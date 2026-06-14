// The subscribe-with-reconnect-recovery supervisor (P6.8 L1, slice 052, layer C).
//
// Drives the live delta stream and recovers it across the daemon's lag-closes (§11.1 degraded gate /
// §11.7 honest degradation — never stale-as-live). The daemon makes a subscribe connection terminal
// and closes it on lag; `ProjectionDelta` carries no `seq`, so a gap is client-undetectable — the
// only correct recovery is a fresh ground-truth reset: reconnect → re-`get_projection` (snapshot) →
// re-subscribe (daemon LESSON 12). NOT client gap-fill.
//
// Pure-orchestration over injected deps (the transport, the connection setter, an injectable backoff,
// an abort check) → fully unit-testable with fakes (no real socket/timer).
import type { ProjectionDelta } from "../contracts/index";
import type { ConnectionState } from "../connection/state";

export interface SubscriptionSupervisorDeps {
  /** Open a fresh subscribe stream (a new dedicated connection each call). */
  subscribe: () => AsyncIterable<ProjectionDelta>;
  /** Apply one delta to the live read cache (re-render). */
  onDelta: (delta: ProjectionDelta) => void;
  /** Re-fetch the projection snapshot (re-`get_projection`) — the ground-truth reset after a close. */
  refetch: () => Promise<void>;
  /** Drive the connection state (the Shell's setter; emits a legal canTransition walk). */
  setConnection: (state: ConnectionState) => void;
  /** Bounded backoff before a reconnect attempt (injectable so tests are timer-free). */
  delay: (attempt: number) => Promise<void>;
  /** False once the effect is torn down (unmount) — the loop exits, no leaked subscription. */
  shouldContinue: () => boolean;
}

/**
 * Run the subscribe loop with automatic reconnect recovery until `shouldContinue()` is false.
 *
 * Each iteration: open a stream and apply its deltas until it ends (a clean lag-close) or throws (a
 * transport error) — either is "the stream ended, recover" (never a crash). Then: mark `disconnected`
 * (§11.1 degrade → `canSubmitIntent` false), back off, mark `reconnecting`, re-`get_projection` the
 * snapshot, and on success mark `connected` (the next loop re-subscribes). A failed refetch stays
 * `disconnected` (never claims `connected` over a daemon it can't reach — no stale-as-live §11.7) and
 * the loop retries. **Recovery order note:** the snapshot re-fetch precedes the re-subscribe so deltas
 * always resume against a fresh baseline (safer than the literal re-subscribe→re-fetch); the cost is a
 * tiny re-subscribe gap window — a delta in the snapshot↔subscribe interval is missed until the next
 * delta for that row — an accepted MVP limitation (self-heals; the seq-less close-on-lag model can't
 * detect gaps anyway, which is why the re-`get_projection` is the ground-truth reset).
 */
export async function runSubscriptionSupervisor(
  deps: SubscriptionSupervisorDeps,
): Promise<void> {
  let attempt = 0;
  while (deps.shouldContinue()) {
    try {
      for await (const delta of deps.subscribe()) {
        if (!deps.shouldContinue()) return;
        deps.onDelta(delta);
      }
      // the stream closed cleanly (lag-close) — fall through to recover.
    } catch {
      // a transport error on the stream — recover, never crash (honest degrade §11.7).
    }
    if (!deps.shouldContinue()) return;

    deps.setConnection("disconnected"); // §11.1 degrade — canSubmitIntent → false
    attempt += 1;
    await deps.delay(attempt); // bounded backoff (don't hammer the daemon)
    if (!deps.shouldContinue()) return;

    deps.setConnection("reconnecting");
    try {
      await deps.refetch(); // re-get_projection — the fresh ground-truth snapshot
      deps.setConnection("connected"); // resume → the next loop re-subscribes
      attempt = 0;
    } catch {
      // the daemon is still unreachable — stay degraded (no stale-as-live); the loop retries.
      deps.setConnection("disconnected");
    }
  }
}

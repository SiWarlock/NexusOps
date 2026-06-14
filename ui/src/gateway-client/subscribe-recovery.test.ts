import { describe, it, expect, vi } from "vitest";
import { runSubscriptionSupervisor } from "./subscribe-recovery";
import { canTransition, type ConnectionState } from "../connection/state";
import type { ProjectionDelta } from "../contracts/index";

const sessionDelta = (id: string): ProjectionDelta => ({
  projection: "Session",
  kind: "upsert",
  id,
});

/** A subscribe that yields the given deltas then ENDS (the daemon lag-close). */
async function* closingStream(
  deltas: ProjectionDelta[],
): AsyncGenerator<ProjectionDelta> {
  for (const d of deltas) yield d;
  // returns → the stream is closed (lag-close), driving the recovery.
}

describe("runSubscriptionSupervisor — reconnect recovery (§11.1/§11.7)", () => {
  it("stream_close_triggers_reconnect_resubscribe_refetch — degrade → backoff → re-fetch → connected → re-subscribe", async () => {
    const states: ConnectionState[] = [];
    const onDelta = vi.fn();
    const refetch = vi.fn().mockResolvedValue(undefined);
    const delay = vi.fn().mockResolvedValue(undefined);
    let subscribeCount = 0;
    const aborted = { value: false };

    await runSubscriptionSupervisor({
      subscribe: () => {
        subscribeCount += 1;
        // first subscription closes (drives one recovery); after the re-subscribe, abort so the
        // loop terminates instead of spinning forever.
        if (subscribeCount >= 2) aborted.value = true;
        return closingStream([sessionDelta("s1")]);
      },
      onDelta,
      refetch,
      setConnection: (s) => states.push(s),
      delay,
      shouldContinue: () => !aborted.value,
    });

    expect(onDelta).toHaveBeenCalledTimes(1); // the one delta from the first stream
    expect(delay).toHaveBeenCalledTimes(1); // one backoff before the reconnect
    expect(refetch).toHaveBeenCalledTimes(1); // re-get_projection snapshot
    expect(subscribeCount).toBe(2); // re-subscribed after recovery
    // the recovery drove: disconnected (degrade) → reconnecting → connected.
    expect(states).toEqual(["disconnected", "reconnecting", "connected"]);
  });

  it("recovery_respects_legal_transitions — every emitted hop is a legal canTransition walk", async () => {
    const states: ConnectionState[] = [];
    let subscribeCount = 0;
    const aborted = { value: false };

    await runSubscriptionSupervisor({
      subscribe: () => {
        subscribeCount += 1;
        if (subscribeCount >= 2) aborted.value = true;
        return closingStream([]);
      },
      onDelta: vi.fn(),
      refetch: vi.fn().mockResolvedValue(undefined),
      setConnection: (s) => states.push(s),
      delay: vi.fn().mockResolvedValue(undefined),
      shouldContinue: () => !aborted.value,
    });

    // walk from the supervisor's starting point "connected" (the load already confirmed it).
    let prev: ConnectionState = "connected";
    for (const s of states) {
      expect(canTransition(prev, s)).toBe(true);
      prev = s;
    }
  });

  it("a refetch failure stays degraded (disconnected), then the loop retries", async () => {
    const states: ConnectionState[] = [];
    let subscribeCount = 0;
    let refetchCount = 0;
    const aborted = { value: false };

    await runSubscriptionSupervisor({
      subscribe: () => {
        subscribeCount += 1;
        if (subscribeCount >= 2) aborted.value = true; // stop after the first recovery attempt
        return closingStream([]);
      },
      onDelta: vi.fn(),
      refetch: () => {
        refetchCount += 1;
        return Promise.reject(new Error("daemon still down"));
      },
      setConnection: (s) => states.push(s),
      delay: vi.fn().mockResolvedValue(undefined),
      shouldContinue: () => !aborted.value,
    });

    expect(refetchCount).toBe(1);
    // a failed refetch never claims "connected" — it stays disconnected (no stale-as-live §11.7).
    expect(states).toEqual(["disconnected", "reconnecting", "disconnected"]);
    expect(states).not.toContain("connected");
  });

  it("recovery_stops_on_abort — the supervisor exits when shouldContinue is false (no leaked loop)", async () => {
    const subscribe = vi.fn(() => closingStream([]));
    await runSubscriptionSupervisor({
      subscribe,
      onDelta: vi.fn(),
      refetch: vi.fn().mockResolvedValue(undefined),
      setConnection: vi.fn(),
      delay: vi.fn().mockResolvedValue(undefined),
      shouldContinue: () => false, // already aborted (e.g. unmounted) → never subscribes
    });
    expect(subscribe).not.toHaveBeenCalled();
  });
});

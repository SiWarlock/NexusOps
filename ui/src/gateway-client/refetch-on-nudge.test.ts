// Coalesced refetch-on-nudge (P6.8, ui-059 L1) — unit pins for the nudge coalescer.
//
// The daemon's ApprovalQueue `ProjectionDelta` is a `row:None` id-NUDGE (gateway/pipeline.rs:79), so
// the live queue consumes it by re-reading the snapshot (`get_projection`), NOT a row-apply reducer.
// This helper coalesces a burst of nudges into a bounded number of re-reads: at-most-one-in-flight +
// one-trailing. Timer-free → driven here by injected deferred promises (no fake clock).
import { describe, it, expect, vi } from "vitest";
import { createNudgeCoalescer } from "./refetch-on-nudge";

/** A manually-settled promise so a test drives in-flight/settle deterministically (no timers). */
function deferred(): {
  promise: Promise<void>;
  resolve: () => void;
  reject: (e: unknown) => void;
} {
  let resolve!: () => void;
  let reject!: (e: unknown) => void;
  const promise = new Promise<void>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

/** Flush the microtask queue a few rounds (the `.catch().finally()` chain spans several hops). */
async function flush(): Promise<void> {
  for (let i = 0; i < 6; i++) await Promise.resolve();
}

describe("refetch-on-nudge coalescer (ui-059 L1)", () => {
  it("idle_nudge_reads_immediately — a nudge with no in-flight read triggers exactly one read", async () => {
    const ds: ReturnType<typeof deferred>[] = [];
    const read = vi.fn(() => {
      const d = deferred();
      ds.push(d);
      return d.promise;
    });
    const c = createNudgeCoalescer(read);

    c.nudge();
    expect(read).toHaveBeenCalledTimes(1);
    ds[0]!.resolve();
    await flush();
    expect(read).toHaveBeenCalledTimes(1); // no further nudges → no further reads
  });

  it("refetch_on_nudge_coalesces_burst — N nudges during one in-flight read collapse to ONE trailing re-read", async () => {
    const ds: ReturnType<typeof deferred>[] = [];
    const read = vi.fn(() => {
      const d = deferred();
      ds.push(d);
      return d.promise;
    });
    const c = createNudgeCoalescer(read);

    c.nudge(); // read #1 starts (in-flight)
    expect(read).toHaveBeenCalledTimes(1);

    c.nudge();
    c.nudge();
    c.nudge(); // 3 nudges during the in-flight read → collapse to ONE trailing
    expect(read).toHaveBeenCalledTimes(1); // still only the in-flight read (no spam)

    ds[0]!.resolve(); // read #1 completes
    await flush();
    expect(read).toHaveBeenCalledTimes(2); // exactly ONE trailing re-read (the burst coalesced)

    ds[1]!.resolve();
    await flush();
    expect(read).toHaveBeenCalledTimes(2); // settled — no further reads
  });

  it("refetch_failure_keeps_coalescer_live — a rejected read is swallowed (no unhandled reject), the trailing read still fires, and a later nudge isn't wedged", async () => {
    // spec(§11.7) — the honest degrade is owned by the PORT's read-path (get_projection→invokeRead→
    // markDisconnected, LESSON §22) which already ran before the rejection reaches the coalescer; the
    // coalescer must merely SURVIVE the rejection so a transient refetch fault never permanently
    // wedges the queue's liveness (and never leaks an unhandled rejection).
    const ds: ReturnType<typeof deferred>[] = [];
    const read = vi.fn(() => {
      const d = deferred();
      ds.push(d);
      return d.promise;
    });
    const c = createNudgeCoalescer(read);

    c.nudge(); // read #1 (in-flight)
    c.nudge(); // trailing queued
    ds[0]!.reject(new Error("transport fault")); // read #1 FAILS
    await flush();
    expect(read).toHaveBeenCalledTimes(2); // the trailing read STILL fired despite the failure

    ds[1]!.resolve();
    await flush();

    c.nudge(); // a fresh nudge after settle
    expect(read).toHaveBeenCalledTimes(3); // not wedged — still re-reads
  });

  it("nudge_during_trailing_read_chains_a_further_read — coalescing spans the primary AND trailing slots", async () => {
    // The N>2 path: a nudge arriving while the TRAILING read is itself in flight schedules one more,
    // so a continuous stream of nudges never wedges — each read covers everything since it started.
    const ds: ReturnType<typeof deferred>[] = [];
    const read = vi.fn(() => {
      const d = deferred();
      ds.push(d);
      return d.promise;
    });
    const c = createNudgeCoalescer(read);

    c.nudge(); // read #1 (in-flight)
    c.nudge(); // trailing queued
    ds[0]!.resolve(); // read #1 done → read #2 (the trailing) starts
    await flush();
    expect(read).toHaveBeenCalledTimes(2);

    c.nudge(); // a nudge DURING read #2 → a fresh trailing
    expect(read).toHaveBeenCalledTimes(2); // still in read #2 (coalesced, not spammed)
    ds[1]!.resolve(); // read #2 done → read #3 (the new trailing) starts
    await flush();
    expect(read).toHaveBeenCalledTimes(3); // chained across slots — never wedged

    ds[2]!.resolve();
    await flush();
    expect(read).toHaveBeenCalledTimes(3); // settled
  });
});

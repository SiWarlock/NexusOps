// Coalesced refetch-on-nudge (P6.8, ui-059 L1).
//
// A daemon `ProjectionDelta` for ApprovalQueue is a `row:None` id-NUDGE (gateway/pipeline.rs:79) — it
// signals "this projection changed", not the new row. So a live ApprovalQueue consumes a nudge by
// RE-READING the snapshot (`get_projection`), NOT a row-apply reducer (which no-ops on `row:None` — the
// 052 Session gap that only passes because the Mock fixture carries a row). This pure helper coalesces a
// burst of nudges into a bounded number of re-reads — at-most-one-in-flight + one-trailing — so the
// final state is never missed and the daemon is never hammered. Timer-free → unit-testable with an
// injected async `read` (no fake clock).

export interface NudgeCoalescer {
  /** Signal that a re-read is due. While a read is in flight, any nudge schedules exactly ONE trailing
   *  read to run after it completes (collapsing a whole burst), so at most 2 reads cover any burst. */
  nudge(): void;
}

/**
 * Build a coalescer over an async `read` (the re-read-the-snapshot work). Semantics:
 *  - idle + nudge → run `read` immediately;
 *  - in-flight + nudge(s) → set a single trailing flag; after the in-flight read settles, run `read`
 *    exactly once more, then clear the flag — so N nudges during one read collapse to one trailing read.
 *
 * A REJECTED read is swallowed here: the honest degrade is owned by the PORT's read-path
 * (`get_projection`→`invokeRead`→`markDisconnected` on a transport/boundary fault, §11.7 / LESSON §22)
 * which has already run before the rejection reaches us. Swallowing only keeps the coalescer alive so a
 * transient refetch fault never permanently wedges the queue's liveness — and never leaks an unhandled
 * rejection. It does NOT mask the degrade (the port already degraded the connection).
 */
export function createNudgeCoalescer(read: () => Promise<void>): NudgeCoalescer {
  let inFlight = false;
  let trailing = false;

  const pump = (): void => {
    inFlight = true;
    void read()
      .catch(() => {
        // honest degrade owned by the port (markDisconnected already fired) — swallow to survive.
      })
      .finally(() => {
        inFlight = false;
        if (trailing) {
          trailing = false;
          pump();
        }
      });
  };

  return {
    nudge(): void {
      if (inFlight) {
        trailing = true; // collapse all nudges during an in-flight read into ONE trailing re-read
        return;
      }
      pump();
    },
  };
}

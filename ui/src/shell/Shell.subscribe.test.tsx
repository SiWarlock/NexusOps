// @vitest-environment jsdom
//
// Layer C integration (P6.8 L1, slice 052) — "live data stays live": a streamed Session delta
// re-renders the cockpit. A dedicated fake gateway (extends MockGatewayPort for the reads) whose
// subscribe yields a status-CHANGING delta then stays open; the Shell's subscribe-effect →
// supervisor → delta-reducer → setData applies it and the new session renders. (The Mock's own
// subscribe streams a benign no-op delta; this proves a visible live update.)
import { describe, it, expect, afterEach, vi } from "vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";

// xterm is a canvas lib (not jsdom-friendly) — mirror Shell.test.tsx's mock.
vi.mock("@xterm/xterm", () => ({
  Terminal: class {
    open() {}
    write() {}
    loadAddon() {}
    dispose() {}
    onData() {
      return { dispose() {} };
    }
    resize() {}
  },
}));
vi.mock("@xterm/addon-fit", () => ({
  FitAddon: class {
    fit() {}
    activate() {}
    dispose() {}
  },
}));

import { Shell } from "./Shell";
import { MockGatewayPort } from "../gateway-client/mock";
import type {
  ApprovalQueuePage,
  ApprovalQueueRow,
  ProjectionDelta,
  ProjectionName,
  ProjectionPageByName,
} from "../contracts/index";
import type {
  ProjectionScope,
  ProjectionPageParams,
  SubscribeParams,
} from "../gateway-client/types";
import { makeApprovalRow } from "../projections/fixtures/proj_approval_queue";
import { projectActivityFixture } from "../projections/fixtures/proj_project_activity";

/** A gateway that streams ONE caller-chosen delta then stays open (a live stream). Reuses the
 *  MockGatewayPort reads/connection; only `subscribe` is overridden. */
class LiveDeltaGateway extends MockGatewayPort {
  constructor(private readonly testDelta: ProjectionDelta) {
    super();
  }
  async *subscribe(): AsyncIterable<ProjectionDelta> {
    yield this.testDelta;
    await new Promise<void>(() => {
      /* stay open — a live stream */
    });
  }
}

afterEach(cleanup);

describe("Shell live subscribe (Layer C — live data stays live)", () => {
  it("applies a streamed Session upsert delta — a new session renders live", async () => {
    // a delta inserting a NEW session in the default active project (project_fixture_1).
    const liveDelta: ProjectionDelta = {
      projection: "Session",
      kind: "upsert",
      row: {
        session_id: "session_live_new",
        status: "active",
        title: "Live new session",
        project_id: projectActivityFixture.rows[0]!.project_id,
      },
    };
    const { container } = render(
      <Shell gateway={new LiveDeltaGateway(liveDelta)} />,
    );

    // the streamed delta inserts the session into the live read cache → it renders (sidebar tree).
    await waitFor(() => {
      expect(
        container.querySelector('[data-item-id="Session:session_live_new"]'),
      ).not.toBeNull();
    });
  });
});

// ── ui-059 — the live ApprovalQueue subscription (refetch-on-nudge + the 2nd stream's recovery) ──────
// A daemon ApprovalQueue delta is a `row:None` id-NUDGE → the live queue must RE-READ get_projection,
// NOT apply the (absent) delta row. These integration pins drive a daemon-shaped Mock through the real
// Shell 2nd-subscribe effect.

/** A gateway that holds the ApprovalQueue `row:None` nudge until the test releases it (race-free baseline
 *  capture), then serves an AUGMENTED ApprovalQueue (one extra PENDING approval) on the post-nudge
 *  re-read. A row-apply reducer would no-op on the absent row → the extra approval would never count;
 *  only refetch-on-nudge surfaces it (the waiting badge increments). Session stays healthy via super. */
class GatedApprovalRefetchGateway extends MockGatewayPort {
  approvalReads = 0;
  private nudgeReleased = false;
  private releaseNudgeFn!: () => void;
  private readonly nudgeGate: Promise<void>;
  constructor(private readonly extra: ApprovalQueueRow) {
    super();
    this.nudgeGate = new Promise<void>((resolve) => {
      this.releaseNudgeFn = resolve;
    });
  }
  releaseNudge(): void {
    this.nudgeReleased = true; // the live change is now visible to subsequent re-reads
    this.releaseNudgeFn();
  }
  async *subscribe(params: SubscribeParams): AsyncIterable<ProjectionDelta> {
    if (params.projection === "ApprovalQueue") {
      await this.nudgeGate; // hold the nudge until the test captures the baseline
      yield { projection: "ApprovalQueue", kind: "upsert", id: "appr_live_nudge" }; // row:None
      await new Promise<void>(() => {
        /* stay open — a live stream; the never-resolving promise is abandoned at test teardown. */
      });
      return;
    }
    yield* super.subscribe(params); // Session: the mock's benign delta + stay open
  }
  async get_projection<K extends ProjectionName>(
    name: K,
    scope?: ProjectionScope,
    page?: ProjectionPageParams,
  ): Promise<ProjectionPageByName[K]> {
    const result = await super.get_projection(name, scope, page);
    if (name === "ApprovalQueue") {
      this.approvalReads++;
      // The augmented approval appears ONLY after the nudge is released (the live change) — so ANY
      // pre-nudge read (the load, plus any recovery read) is the base set, regardless of count. This
      // ties the augmentation to the actual nudge, not a fragile read-index threshold.
      if (this.nudgeReleased) {
        const aq = result as ApprovalQueuePage;
        return { ...aq, rows: [...aq.rows, this.extra] } as ProjectionPageByName[K];
      }
    }
    return result;
  }
}

/** A gateway whose FIRST ApprovalQueue subscribe lag-closes (→ the supervisor degrades + recovers via
 *  re-get_projection), then stays open. Session stays healthy throughout. */
class RecoveringApprovalStreamGateway extends MockGatewayPort {
  private aqSubscribes = 0;
  async *subscribe(params: SubscribeParams): AsyncIterable<ProjectionDelta> {
    if (params.projection === "ApprovalQueue") {
      this.aqSubscribes += 1;
      if (this.aqSubscribes === 1) return; // first stream lag-closes → supervisor degrades + recovers
      await new Promise<void>(() => {
        /* the re-subscribe stays open (recovered) */
      });
      return;
    }
    yield* super.subscribe(params); // Session stays healthy (the degrade comes ONLY from ApprovalQueue)
  }
}

describe("Shell live ApprovalQueue subscribe (ui-059)", () => {
  it("approvalqueue_nudge_refetches_not_row_apply", async () => {
    // the lead-mandated pin: a row:None nudge drives a REFETCH whose rows replace the queue, NOT a
    // row-apply of the (absent) delta row. The extra pending approval can ONLY appear via the re-read.
    const extra = makeApprovalRow({
      approval_id: "approval_live_refetched",
      project_id: projectActivityFixture.rows[0]!.project_id, // a real project → counts in the switcher
      status: "awaiting_approval", // pending → +1 waitingOnYou
    });
    const gateway = new GatedApprovalRefetchGateway(extra);
    render(<Shell gateway={gateway} />);

    // load settles → the baseline waiting badge (the nudge is still gated → NO refetch yet).
    const badge = await screen.findByTestId("sidebar-waiting-badge");
    const baseline = Number(badge.textContent);
    const readsBeforeNudge = gateway.approvalReads;

    gateway.releaseNudge(); // the row:None nudge fires → coalesced refetch → augmented re-read

    await waitFor(() => {
      // the nudge caused a RE-READ (not a row-apply)…
      expect(gateway.approvalReads).toBeGreaterThan(readsBeforeNudge);
      // …and the re-read's extra pending approval is reflected in the live queue (badge +1).
      expect(
        Number(screen.getByTestId("sidebar-waiting-badge").textContent),
      ).toBe(baseline + 1);
    });
  });

  it("approvalqueue_subscribe_recovers_on_lag_close", async () => {
    // the ApprovalQueue stream reuses the 052 supervisor: a lag-close → reconnect → re-get_projection
    // (the ground-truth snapshot reset) → re-subscribe. The recovery refetch is observable as a 2nd
    // ApprovalQueue read (the 052 mechanism, now on the 2nd stream).
    const gateway = new RecoveringApprovalStreamGateway();
    const readSpy = vi.spyOn(gateway, "get_projection");
    render(<Shell gateway={gateway} />);

    await waitFor(
      () => {
        const aqReads = readSpy.mock.calls.filter(
          (c) => c[0] === "ApprovalQueue",
        ).length;
        expect(aqReads).toBeGreaterThanOrEqual(2); // load + the recovery refetch
      },
      { timeout: 3000 },
    );
  });
});

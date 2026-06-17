// @vitest-environment jsdom
//
// Layer C integration (P6.8) — "live data stays live": a daemon ProjectionDelta keeps the cockpit
// current. Both the Session (ui-062) and ApprovalQueue (ui-059) streams consume a `row:None` id-NUDGE
// via REFETCH-ON-NUDGE (a coalesced re-read of get_projection), NOT a row-apply reducer (which no-ops
// on the absent row — LESSON §29). A dedicated fake gateway (extends MockGatewayPort for the reads)
// holds the nudge until the test captures a baseline, then serves an AUGMENTED page on the re-read so
// the live change is observable only via the refetch (proving it's not a row-apply).
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
  SessionProjectionPage,
  SessionRow,
} from "../contracts/index";
import type {
  ProjectionScope,
  ProjectionPageParams,
  SubscribeParams,
} from "../gateway-client/types";
import { makeApprovalRow } from "../projections/fixtures/proj_approval_queue";
import { projectActivityFixture } from "../projections/fixtures/proj_project_activity";

/** A gateway that holds the Session `row:None` nudge until the test releases it (race-free baseline
 *  capture), then serves an AUGMENTED Session page (one extra session) on the post-nudge re-read. A
 *  row-apply reducer would no-op on the absent row → the extra session would never appear; only
 *  refetch-on-nudge surfaces it (ui-062 — the §29 pattern generalized from ApprovalQueue to Session).
 *  ApprovalQueue stays healthy via super. */
class GatedSessionRefetchGateway extends MockGatewayPort {
  sessionReads = 0;
  private nudgeReleased = false;
  private releaseNudgeFn!: () => void;
  private readonly nudgeGate: Promise<void>;
  constructor(private readonly extra: SessionRow) {
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
    if (params.projection === "Session") {
      await this.nudgeGate; // hold the nudge until the test captures the baseline
      yield { projection: "Session", kind: "upsert", id: "sess_live_nudge" }; // row:None
      await new Promise<void>(() => {
        /* stay open — a live stream; the never-resolving promise is abandoned at test teardown. */
      });
      return;
    }
    yield* super.subscribe(params); // ApprovalQueue: the mock's benign delta + stay open
  }
  async get_projection<K extends ProjectionName>(
    name: K,
    scope?: ProjectionScope,
    page?: ProjectionPageParams,
  ): Promise<ProjectionPageByName[K]> {
    const result = await super.get_projection(name, scope, page);
    if (name === "Session") {
      this.sessionReads++;
      // The augmented session appears ONLY after the nudge is released (the live change) — so ANY
      // pre-nudge read (the load + any recovery read) is the base set, regardless of count.
      if (this.nudgeReleased) {
        const s = result as SessionProjectionPage;
        return { ...s, rows: [...s.rows, this.extra] } as ProjectionPageByName[K];
      }
    }
    return result;
  }
}

afterEach(cleanup);

describe("Shell live Session subscribe (ui-062 — refetch-on-nudge)", () => {
  it("session_subscribe_refetches_on_row_none_nudge", async () => {
    // ui-062: a daemon Session delta is a `row:None` id-NUDGE (SessionStarted/Failed/Recovered) → the
    // live list must REFETCH get_projection("Session"), NOT apply the absent delta row (the removed
    // applySessionDelta no-op'd on row:None, §29). The new session can ONLY appear via the re-read.
    const extra: SessionRow = {
      session_id: "session_live_refetched",
      status: "active",
      display_name: "Live new session",
      project_id: projectActivityFixture.rows[0]!.project_id, // the default active project → renders in the tree
    };
    const gateway = new GatedSessionRefetchGateway(extra);
    const { container } = render(<Shell gateway={gateway} />);

    // load settles → the nudge is still gated → the new session is NOT present yet (NO refetch).
    await screen.findByTestId("sidebar-waiting-badge");
    expect(
      container.querySelector('[data-item-id="Session:session_live_refetched"]'),
    ).toBeNull();
    const readsBeforeNudge = gateway.sessionReads;

    gateway.releaseNudge(); // the row:None nudge fires → coalesced refetch → augmented re-read

    await waitFor(() => {
      // the nudge caused a RE-READ (not a row-apply)…
      expect(gateway.sessionReads).toBeGreaterThan(readsBeforeNudge);
      // …and the re-read's extra session is now in the live tree.
      expect(
        container.querySelector('[data-item-id="Session:session_live_refetched"]'),
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

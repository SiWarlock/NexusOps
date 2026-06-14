// @vitest-environment jsdom
//
// Layer C integration (P6.8 L1, slice 052) — "live data stays live": a streamed Session delta
// re-renders the cockpit. A dedicated fake gateway (extends MockGatewayPort for the reads) whose
// subscribe yields a status-CHANGING delta then stays open; the Shell's subscribe-effect →
// supervisor → delta-reducer → setData applies it and the new session renders. (The Mock's own
// subscribe streams a benign no-op delta; this proves a visible live update.)
import { describe, it, expect, afterEach, vi } from "vitest";
import { cleanup, render, waitFor } from "@testing-library/react";

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
import type { ProjectionDelta } from "../contracts/index";
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

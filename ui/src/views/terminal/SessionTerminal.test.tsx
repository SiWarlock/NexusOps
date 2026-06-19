// @vitest-environment jsdom
import { describe, it, expect, vi, afterEach } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";

// Mock xterm (display-only #9 host is canvas — visual-gated, not unit-rendered).
// CLASS mocks since Terminal/FitAddon are `new`-ed by TerminalDisplay.
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

import { SessionTerminal } from "./SessionTerminal";
import { MockGatewayPort } from "../../gateway-client/mock";
import { sessionPageFixture } from "../../projections/fixtures/proj_session";
import { describeStatus } from "../../status/descriptors";

afterEach(cleanup);

const sessionById = (id: string) =>
  sessionPageFixture.rows.find((s) => s.session_id === id)!;

const props = (id: string) => ({
  session: sessionById(id),
  sessions: sessionPageFixture.rows,
  projects: [],
  gateway: new MockGatewayPort(),
});

describe("SessionTerminal — terminal-well gating (§6.4 / §9.1)", () => {
  it("session_terminal_renders_terminaldisplay_with_stream", () => {
    // spec(§6.4): a LIVE session WITH a terminal stream (terminalId in the display
    // side-map, fixture_3) renders the xterm TerminalDisplay, not the placeholder.
    render(<SessionTerminal {...props("session_fixture_3")} />);
    expect(screen.getByTestId("terminal-xterm")).toBeTruthy();
    expect(screen.queryByTestId("terminal-well-pending")).toBeNull();
  });

  it("session_terminal_status_from_projection_not_stream", () => {
    // spec(§9.1 #9 pin2): the well's session status comes from the Session
    // PROJECTION (session.status) — never derived from the terminal output bytes.
    // The header StatusPill shows the projection status even while the well streams.
    render(<SessionTerminal {...props("session_fixture_3")} />);
    const label = describeStatus("Session", sessionById("session_fixture_3").status).label;
    expect(screen.getByText(label)).toBeTruthy();
  });

  it("keeps_placeholder_when_no_terminal", () => {
    // spec(§6.4): a LIVE session WITHOUT a terminal stream (no terminalId in the
    // side-map — fixture_1) keeps the HONEST placeholder; transcript lines are
    // never invented.
    render(<SessionTerminal {...props("session_fixture_1")} />);
    expect(screen.getByTestId("terminal-well-pending")).toBeTruthy();
    expect(screen.queryByTestId("terminal-xterm")).toBeNull();
  });

  it("renders_terminal_and_permission_card_together", () => {
    // spec(§11.5/§6.4): a live session that is ALSO waiting on permission
    // (fixture_2: terminalId + waiting_on_permission) renders BOTH the streaming
    // xterm well AND the (disabled, intent-seam-gated) permission card — the real
    // cockpit's "output behind an approval" case.
    render(<SessionTerminal {...props("session_fixture_2")} />);
    expect(screen.getByTestId("terminal-xterm")).toBeTruthy();
    expect(screen.getByTestId("terminal-permission-prompt")).toBeTruthy();
  });

  it("session_status_terminal_renders_honest_ended_state", () => {
    // spec(§9.1 #9): an ENDED session (terminal status — fixture_4 completed) shows
    // an HONEST ended state sourced from session.status (the projection), NOT a
    // faked "still running" and NOT a live terminal. exit_code/signal detail has no
    // frozen UI-readable source yet (deferred, P4).
    render(<SessionTerminal {...props("session_fixture_4")} />);
    expect(screen.getByTestId("terminal-ended")).toBeTruthy();
    expect(screen.queryByTestId("terminal-xterm")).toBeNull();
    expect(screen.queryByTestId("terminal-well-pending")).toBeNull();
  });
});

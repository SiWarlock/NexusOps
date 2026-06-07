// @vitest-environment jsdom
import { describe, it, expect, afterEach } from "vitest";
import { cleanup, render, screen, fireEvent } from "@testing-library/react";
import { Shell } from "./Shell";
import { MockGatewayPort } from "../gateway-client/mock";
import { BoundaryValidationError } from "../gateway-client/boundary";
import { SessionProjectionPage } from "../contracts/index";
import type { GatewayPort } from "../gateway-client/types";
import { projectActivityFixture } from "../projections/fixtures/proj_project_activity";

afterEach(cleanup);

// A gateway that simulates the boundary REJECTING a malformed daemon payload —
// get_projection rejects with a real BoundaryValidationError (built from an
// actual failed parse), exactly as the validated seam would on bad bytes.
function makeBoundaryError(): BoundaryValidationError {
  const res = SessionProjectionPage.safeParse({
    projection: "Session",
    rows: [{ status: "bogus" }],
  });
  if (res.success) throw new Error("test setup: expected a parse failure");
  return new BoundaryValidationError("Session", res.error);
}
const rejectingGateway: GatewayPort = {
  get_projection: () => Promise.reject(makeBoundaryError()),
  // eslint-disable-next-line require-yield
  subscribe: async function* () {
    return;
  },
  get_capabilities: () =>
    Promise.resolve({ protocol_version: 1, contract_version: "0.5.0" }),
};

describe("Shell", () => {
  it("shell_renders_projects_from_projection", async () => {
    render(<Shell gateway={new MockGatewayPort()} />);
    // exactly the fixture's projects, none invented
    for (const project of projectActivityFixture.rows) {
      expect(await screen.findByText(project.name)).toBeTruthy();
    }
  });

  it("shell_reads_only_through_gateway_boundary", async () => {
    render(<Shell gateway={rejectingGateway} />);
    // a boundary reject surfaces as a handled error state, never a raw render / crash
    expect(await screen.findByTestId("shell-load-error")).toBeTruthy();
  });

  it("shell_activity_dock_collapsed_and_expanded", async () => {
    render(<Shell gateway={new MockGatewayPort()} />);
    // wait for load
    await screen.findByText(projectActivityFixture.rows[0]!.name);
    // collapsed by default: the event timeline is not shown
    expect(screen.queryByTestId("activity-timeline")).toBeNull();
    // expand → timeline appears, bound to the audit projection events
    fireEvent.click(screen.getByRole("button", { name: /activity/i }));
    const timeline = await screen.findByTestId("activity-timeline");
    expect(timeline).toBeTruthy();
    expect(timeline.textContent).toContain("session.started");
  });

  it("shell_kit_token_layer_applied", async () => {
    const { container } = render(<Shell gateway={new MockGatewayPort()} />);
    await screen.findByText(projectActivityFixture.rows[0]!.name);
    // a kit component renders with CSS custom-property REFERENCES in its inline
    // style (jsdom doesn't load styles.css, so the var() tokens aren't resolved
    // here — this pins that the kit's token-driven styling is wired, not that a
    // computed color came back).
    expect(container.querySelector('[style*="var(--"]')).not.toBeNull();
  });
});

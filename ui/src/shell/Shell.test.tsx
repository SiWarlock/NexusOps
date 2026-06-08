// @vitest-environment jsdom
import { describe, it, expect, afterEach } from "vitest";
import { act, cleanup, render, screen, fireEvent, within } from "@testing-library/react";
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
  getConnectionState: () => "connected",
  onConnectionChange: () => () => {},
  reconnect: () => {},
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

  it("shell_connection_change_drives_degraded_banner", async () => {
    // Exercises the REAL subscription path (onConnectionChange → Shell state →
    // banner), not just a prop rerender.
    const mock = new MockGatewayPort();
    render(<Shell gateway={mock} />);
    await screen.findByText(projectActivityFixture.rows[0]!.name);
    expect(screen.queryByRole("alert")).toBeNull(); // connected + compatible → no banner

    act(() => {
      mock.setConnectionState("disconnected");
    });
    expect(await screen.findByRole("alert")).toBeTruthy(); // degraded banner shown
  });

  it("view_switch_mounts_project_graph", async () => {
    const { container } = render(<Shell gateway={new MockGatewayPort()} />);
    await screen.findByText(projectActivityFixture.rows[0]!.name); // loaded
    // Command Center is the default content view
    expect(container.querySelector('[aria-label="Command Center"]')).not.toBeNull();
    expect(screen.queryByTestId("graph-canvas")).toBeNull();
    // switching the content view to Project Graph mounts <ProjectGraph/> for
    // the first project (reachable from the Shell — Step 7.5 entry point)
    fireEvent.click(screen.getByRole("button", { name: /project graph/i }));
    expect(screen.getByTestId("graph-canvas")).toBeTruthy();
    // it's a switch, not a stack — Command Center is unmounted
    expect(container.querySelector('[aria-label="Command Center"]')).toBeNull();
  });

  it("view_switch_mounts_sessions_table", async () => {
    const { container } = render(<Shell gateway={new MockGatewayPort()} />);
    await screen.findByText(projectActivityFixture.rows[0]!.name); // loaded
    // Command Center is the default content view
    expect(container.querySelector('[aria-label="Command Center"]')).not.toBeNull();
    expect(screen.queryByTestId("sessions-table")).toBeNull();
    // selecting Sessions mounts <SessionsTable/>, reachable from the Shell (Step 7.5)
    fireEvent.click(screen.getByRole("button", { name: /sessions/i }));
    expect(screen.getByTestId("sessions-table")).toBeTruthy();
    // switch-not-stack — Command Center is unmounted
    expect(container.querySelector('[aria-label="Command Center"]')).toBeNull();
  });

  it("view_switch_mounts_settings_not_interim_usage", async () => {
    const { container } = render(<Shell gateway={new MockGatewayPort()} />);
    await screen.findByText(projectActivityFixture.rows[0]!.name); // loaded
    // the content-view switch offers Settings, NOT a top-level interim "Usage".
    // (Scoped to the switch group — the TopBar also has a placeholder "Settings".)
    const viewSwitch = screen.getByRole("group", { name: "Content view" });
    expect(within(viewSwitch).getByRole("button", { name: /settings/i })).toBeTruthy();
    expect(within(viewSwitch).queryByRole("button", { name: /usage/i })).toBeNull();
    expect(container.querySelector('[aria-label="Command Center"]')).not.toBeNull();
    // selecting Settings mounts the tablist (switch-not-stack); Usage is now
    // reached via Settings → Usage (the relocation — Step 7.5)
    fireEvent.click(within(viewSwitch).getByRole("button", { name: /settings/i }));
    expect(screen.getByRole("tablist")).toBeTruthy();
    expect(container.querySelector('[aria-label="Command Center"]')).toBeNull();
    fireEvent.click(screen.getByRole("tab", { name: /usage/i }));
    expect(screen.getByTestId("usage-table")).toBeTruthy();
  });

  it("shell_renders_recovery_banner", async () => {
    render(
      <Shell
        gateway={new MockGatewayPort()}
        recovery={{ state: "recovery_failed", affectedSessions: ["session_fixture_2"] }}
      />,
    );
    await screen.findByText(projectActivityFixture.rows[0]!.name); // loaded
    // the post-restart recovery banner is reachable in the shell (Step 7.5),
    // distinct from the transport degraded banner
    const banner = screen.getByTestId("recovery-banner");
    expect(banner.getAttribute("data-recovery-kind")).toBe("recovery_failed");
  });
});

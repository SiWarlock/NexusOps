// @vitest-environment jsdom
import { describe, it, expect, afterEach } from "vitest";
import { act, cleanup, render, screen, fireEvent, within } from "@testing-library/react";
import { Shell } from "./Shell";
import { MockGatewayPort } from "../gateway-client/mock";
import { BoundaryValidationError } from "../gateway-client/boundary";
import { SessionProjectionPage } from "../contracts/index";
import type { GatewayPort } from "../gateway-client/types";
import { projectActivityFixture } from "../projections/fixtures/proj_project_activity";
import { fencingConflictFixture } from "../safety/fixtures";

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

  it("topbar_settings_opens_settings_view", async () => {
    const { container } = render(<Shell gateway={new MockGatewayPort()} />);
    await screen.findByText(projectActivityFixture.rows[0]!.name); // loaded
    expect(container.querySelector('[aria-label="Command Center"]')).not.toBeNull();
    expect(screen.queryByRole("tablist")).toBeNull();
    // §11.2 nav: the TopBar Settings control opens the Settings view (it's the only
    // "Settings" button now — the view-switch dropped it, so no scoping needed)
    fireEvent.click(screen.getByRole("button", { name: /settings/i }));
    expect(screen.getByRole("tablist")).toBeTruthy();
    expect(container.querySelector('[aria-label="Command Center"]')).toBeNull();
  });

  it("view_switch_no_longer_offers_settings", async () => {
    render(<Shell gateway={new MockGatewayPort()} />);
    await screen.findByText(projectActivityFixture.rows[0]!.name); // loaded
    // the content-view switch is content surfaces only — CC / Graph / Sessions
    const viewSwitch = screen.getByRole("group", { name: "Content view" });
    expect(within(viewSwitch).getAllByRole("button").map((b) => b.textContent)).toEqual([
      "Command Center",
      "Project Graph",
      "Sessions",
    ]);
    expect(within(viewSwitch).queryByRole("button", { name: /settings/i })).toBeNull();
  });

  it("settings_still_reachable_and_functional", async () => {
    render(<Shell gateway={new MockGatewayPort()} />);
    await screen.findByText(projectActivityFixture.rows[0]!.name); // loaded
    // Settings (the 6.4c tablist + Usage tab) is reachable + functional via TopBar
    fireEvent.click(screen.getByRole("button", { name: /settings/i }));
    fireEvent.click(screen.getByRole("tab", { name: /usage/i }));
    expect(screen.getByTestId("usage-table")).toBeTruthy();
  });

  it("shell_renders_hard_conflict_card_from_safety_prop", async () => {
    render(
      <Shell
        gateway={new MockGatewayPort()}
        safety={{ conflict: fencingConflictFixture, integrity: null }}
      />,
    );
    await screen.findByText(projectActivityFixture.rows[0]!.name); // loaded
    // the §17 hard-conflict card is reachable in the shell (Step 7.5) with its
    // parked (disabled) resolution control — a distinct safety surface
    const card = screen.getByTestId("hard-conflict-card");
    expect(card.getAttribute("data-conflict-reason")).toBe("fencing_conflict");
    expect(screen.getByTestId("conflict-resolve")).toHaveProperty("disabled", true);
  });

  it("shell_renders_audit_integrity_alert_from_safety_prop", async () => {
    render(
      <Shell
        gateway={new MockGatewayPort()}
        safety={{
          conflict: null,
          integrity: { source: "integrity", kind: "audit_write_failed" },
        }}
      />,
    );
    await screen.findByText(projectActivityFixture.rows[0]!.name); // loaded
    // the §17 fail-closed audit-integrity alert is reachable in the shell (Step
    // 7.5), non-dismissible (its acknowledge is parked/disabled)
    const alert = screen.getByTestId("audit-integrity-alert");
    expect(alert.getAttribute("data-treatment")).toBe("audit_write_failed");
    expect(screen.getByTestId("audit-integrity-acknowledge")).toHaveProperty(
      "disabled",
      true,
    );
  });

  it("shell_safety_clean_by_default_renders_no_safety_surfaces", async () => {
    render(<Shell gateway={new MockGatewayPort()} />);
    await screen.findByText(projectActivityFixture.rows[0]!.name); // loaded
    // default safety fixture is clean → neither surface intrudes (like 6.4d recovered)
    expect(screen.queryByTestId("hard-conflict-card")).toBeNull();
    expect(screen.queryByTestId("audit-integrity-alert")).toBeNull();
  });

  it("shell_active_project_reroots_graph_and_filters_sessions", async () => {
    render(<Shell gateway={new MockGatewayPort()} />);
    await screen.findByText(projectActivityFixture.rows[0]!.name); // loaded (auth-service)
    // default active = the first project; select billing (project_fixture_2) in the switcher
    fireEvent.click(screen.getByRole("button", { name: /billing/i }));
    // (a) the graph re-roots at the active project (billing), not the hardcoded projects[0]
    fireEvent.click(screen.getByRole("button", { name: /project graph/i }));
    fireEvent.click(screen.getByRole("button", { name: /^list$/i }));
    expect(
      screen
        .getByTestId("graph-table")
        .querySelector('[data-item-id="project:project_fixture_2"]'),
    ).not.toBeNull();
    // (b) the Sessions view filters to the active project (billing has 2 sessions)
    fireEvent.click(screen.getByRole("button", { name: /sessions/i }));
    const rows = screen
      .getByTestId("sessions-table")
      .querySelectorAll("tbody tr[data-item-id]");
    expect(rows).toHaveLength(2);
  });

  it("shell_history_nav_round_trips_content", async () => {
    const { container } = render(<Shell gateway={new MockGatewayPort()} />);
    await screen.findByText(projectActivityFixture.rows[0]!.name); // loaded
    // Command Center is the default content view; history has no back yet.
    expect(container.querySelector('[aria-label="Command Center"]')).not.toBeNull();
    expect(screen.getByRole("button", { name: /back/i })).toHaveProperty(
      "disabled",
      true,
    );
    // navigate Command → Graph via the view-switch (a real nav, pushes history)
    fireEvent.click(screen.getByRole("button", { name: /project graph/i }));
    expect(screen.getByTestId("graph-canvas")).toBeTruthy();
    // TopBar Back is now live and returns the Command surface
    fireEvent.click(screen.getByRole("button", { name: /back/i }));
    expect(container.querySelector('[aria-label="Command Center"]')).not.toBeNull();
    expect(screen.queryByTestId("graph-canvas")).toBeNull();
    // TopBar Forward returns Graph (cursor moves, stack intact)
    fireEvent.click(screen.getByRole("button", { name: /forward/i }));
    expect(screen.getByTestId("graph-canvas")).toBeTruthy();
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

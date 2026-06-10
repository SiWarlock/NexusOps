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

// The first project's name renders in BOTH the switcher trigger and the sidebar
// workspace tree (prototype chrome) — the load gate awaits all matches.
const awaitLoaded = () =>
  screen.findAllByText(projectActivityFixture.rows[0]!.name);

// The switcher trigger carries the stable "Switch project" title (the project
// name itself appears in several chrome regions).
const switcherTrigger = () => screen.getByTitle("Switch project");

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
    await awaitLoaded();
    // open the switcher dropdown → it lists EXACTLY the projection's projects (none
    // invented); query within the listbox so the chrome's copies don't double-match
    fireEvent.click(switcherTrigger());
    const listbox = screen.getByRole("listbox");
    for (const project of projectActivityFixture.rows) {
      expect(within(listbox).getByText(project.name)).toBeTruthy();
    }
  });

  it("shell_reads_only_through_gateway_boundary", async () => {
    render(<Shell gateway={rejectingGateway} />);
    // a boundary reject surfaces as a handled error state, never a raw render / crash
    expect(await screen.findByTestId("shell-load-error")).toBeTruthy();
  });

  it("shell_event_dock_collapsed_and_expanded", async () => {
    render(<Shell gateway={new MockGatewayPort()} />);
    await awaitLoaded();
    // collapsed by default: the event timeline is not shown
    expect(screen.queryByTestId("activity-timeline")).toBeNull();
    // expand → timeline appears, bound to the audit projection events (scoped to
    // the active project — auth-service)
    fireEvent.click(screen.getByRole("button", { name: /activity/i }));
    const timeline = await screen.findByTestId("activity-timeline");
    expect(timeline).toBeTruthy();
    expect(timeline.textContent).toContain("session.started");
  });

  it("shell_event_dock_full_audit_navigates", async () => {
    render(<Shell gateway={new MockGatewayPort()} />);
    await awaitLoaded();
    fireEvent.click(screen.getByRole("button", { name: /activity/i }));
    fireEvent.click(screen.getByRole("button", { name: /full audit/i }));
    // the audit view mounts (placeholder surface until the audit slice lands)
    expect(screen.getByTestId("placeholder-audit-trail")).toBeTruthy();
  });

  it("shell_kit_token_layer_applied", async () => {
    const { container } = render(<Shell gateway={new MockGatewayPort()} />);
    await awaitLoaded();
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
    await awaitLoaded();
    expect(screen.queryByRole("alert")).toBeNull(); // connected + compatible → no banner

    act(() => {
      mock.setConnectionState("disconnected");
    });
    expect(await screen.findByRole("alert")).toBeTruthy(); // degraded banner shown
  });

  it("sidebar_nav_mounts_project_graph", async () => {
    const { container } = render(<Shell gateway={new MockGatewayPort()} />);
    await awaitLoaded();
    // Command Center is the default content view
    expect(container.querySelector('[aria-label="Command Center"]')).not.toBeNull();
    expect(screen.queryByTestId("graph-canvas")).toBeNull();
    // the sidebar nav mounts <ProjectGraph/> for the active project (Step 7.5)
    fireEvent.click(screen.getByRole("button", { name: /project graph/i }));
    expect(screen.getByTestId("graph-canvas")).toBeTruthy();
    // it's a switch, not a stack — Command Center is unmounted
    expect(container.querySelector('[aria-label="Command Center"]')).toBeNull();
  });

  it("sidebar_nav_mounts_session_terminal_with_sessions_table", async () => {
    const { container } = render(<Shell gateway={new MockGatewayPort()} />);
    await awaitLoaded();
    expect(screen.queryByTestId("sessions-table")).toBeNull();
    // Session Terminal hosts the sessions table (real projection data) until the
    // daemon terminal channel lands (6.3d/e — flagged, not faked)
    fireEvent.click(screen.getByRole("button", { name: /session terminal/i }));
    expect(
      container.querySelector('[aria-label="Session Terminal"]'),
    ).not.toBeNull();
    expect(screen.getByTestId("sessions-table")).toBeTruthy();
    expect(container.querySelector('[aria-label="Command Center"]')).toBeNull();
  });

  it("topbar_settings_opens_settings_view", async () => {
    const { container } = render(<Shell gateway={new MockGatewayPort()} />);
    await awaitLoaded();
    expect(container.querySelector('[aria-label="Command Center"]')).not.toBeNull();
    expect(screen.queryByRole("tablist")).toBeNull();
    // §11.2 nav: the TopBar Settings control opens the Settings view
    fireEvent.click(screen.getByRole("button", { name: /settings/i }));
    expect(screen.getByRole("tablist")).toBeTruthy();
    expect(container.querySelector('[aria-label="Command Center"]')).toBeNull();
  });

  it("sidebar_nav_offers_prototype_views_not_settings", async () => {
    render(<Shell gateway={new MockGatewayPort()} />);
    await awaitLoaded();
    // the sidebar nav carries the prototype view set; Settings lives in the
    // TopBar only (§11.2 nav model)
    const sidebar = screen.getByRole("navigation", { name: "Sidebar" });
    for (const name of [
      /command center/i,
      /project graph/i,
      /^plan$/i,
      /^editor$/i,
      /session terminal/i,
      /code \/ diff review/i,
      /workflow packs/i,
      /audit trail/i,
    ]) {
      expect(within(sidebar).getByRole("button", { name })).toBeTruthy();
    }
    expect(within(sidebar).queryByRole("button", { name: /settings/i })).toBeNull();
  });

  it("settings_still_reachable_and_functional", async () => {
    render(<Shell gateway={new MockGatewayPort()} />);
    await awaitLoaded();
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
    await awaitLoaded();
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
    await awaitLoaded();
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
    await awaitLoaded();
    // default safety fixture is clean → neither surface intrudes (like 6.4d recovered)
    expect(screen.queryByTestId("hard-conflict-card")).toBeNull();
    expect(screen.queryByTestId("audit-integrity-alert")).toBeNull();
  });

  it("shell_active_project_reroots_graph_and_filters_sessions", async () => {
    render(<Shell gateway={new MockGatewayPort()} />);
    await awaitLoaded();
    // default active = the first project (auth-service); open the switcher dropdown
    // and select billing (project_fixture_2) — selection flows via the popover
    fireEvent.click(switcherTrigger());
    fireEvent.click(screen.getByRole("option", { name: /billing/i }));
    // (a) the graph re-roots at the active project (billing), not the hardcoded projects[0]
    fireEvent.click(screen.getByRole("button", { name: /project graph/i }));
    fireEvent.click(screen.getByRole("button", { name: /^list$/i }));
    expect(
      screen
        .getByTestId("graph-table")
        .querySelector('[data-item-id="project:project_fixture_2"]'),
    ).not.toBeNull();
    // (b) the Session Terminal's table filters to the active project (billing has
    // 2 sessions)
    fireEvent.click(screen.getByRole("button", { name: /session terminal/i }));
    const rows = screen
      .getByTestId("sessions-table")
      .querySelectorAll("tbody tr[data-item-id]");
    expect(rows).toHaveLength(2);
  });

  it("shell_history_nav_round_trips_content", async () => {
    const { container } = render(<Shell gateway={new MockGatewayPort()} />);
    await awaitLoaded();
    // Command Center is the default content view; history has no back yet.
    expect(container.querySelector('[aria-label="Command Center"]')).not.toBeNull();
    expect(screen.getByRole("button", { name: /back/i })).toHaveProperty(
      "disabled",
      true,
    );
    // navigate Command → Graph via the sidebar nav (a real nav, pushes history)
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

  it("shell_sidebar_session_click_opens_terminal", async () => {
    const { container } = render(<Shell gateway={new MockGatewayPort()} />);
    await awaitLoaded();
    // clicking a session row in the workspace tree opens the Session Terminal
    // view targeting it (selection is pure UI state — Lesson §13)
    fireEvent.click(
      container.querySelector('[data-item-id="Session:session_fixture_1"]')!,
    );
    expect(container.querySelector('[aria-label="Session Terminal"]')).not.toBeNull();
  });

  it("shell_sidebar_shows_resume_indicator_from_session_data", async () => {
    const { container } = render(<Shell gateway={new MockGatewayPort()} />);
    // MockGatewayPort sets all projections atomically (one setData), so projects
    // loaded ⇒ sessions present too — this gate is race-free.
    await awaitLoaded();
    // the Shell builds the resume-mode side map from data.sessions and passes it to
    // the Sidebar (Step 7.5 live path): session_fixture_1 carries resume_mode
    // "resumed" → its sidebar item shows the indicator (Lesson §8 — no item widening)
    const item = container.querySelector(
      '.sidebar [data-item-id="Session:session_fixture_1"] [data-resume-mode]',
    );
    expect(item?.getAttribute("data-resume-mode")).toBe("resumed");
  });

  it("shell_renders_recovery_banner", async () => {
    render(
      <Shell
        gateway={new MockGatewayPort()}
        recovery={{ state: "recovery_failed", affectedSessions: ["session_fixture_2"] }}
      />,
    );
    await awaitLoaded();
    // the post-restart recovery banner is reachable in the shell (Step 7.5),
    // distinct from the transport degraded banner
    const banner = screen.getByTestId("recovery-banner");
    expect(banner.getAttribute("data-recovery-kind")).toBe("recovery_failed");
  });
});

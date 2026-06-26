// @vitest-environment jsdom
import { describe, it, expect, afterEach } from "vitest";
import { cleanup, render, screen, fireEvent, within } from "@testing-library/react";
import { Settings } from "./Settings";
import { usageFixture } from "../../projections/fixtures/proj_usage";

afterEach(cleanup);

const renderSettings = () =>
  render(
    <Settings usage={usageFixture.rows} />,
  );

describe("Settings tabbed surface", () => {
  it("renders_tablist_with_aria", () => {
    renderSettings();
    const tablist = screen.getByRole("tablist");
    const tabs = within(tablist).getAllByRole("tab");
    // the prototype's four sections (Notifications stub removed at the rebuild)
    expect(tabs.map((t) => t.textContent)).toEqual([
      "Integrations",
      "Execution profiles",
      "Usage",
      "Security & policy",
    ]);
    // exactly one tab is aria-selected
    const selected = tabs.filter((t) => t.getAttribute("aria-selected") === "true");
    expect(selected).toHaveLength(1);
    // the selected tab is labelled to the visible tabpanel (aria-controls/labelledby)
    const panel = screen.getByRole("tabpanel");
    expect(panel.getAttribute("aria-labelledby")).toBe(selected[0]!.id);
    // every tab's aria-controls resolves to a panel in the DOM (WAI-ARIA — the
    // target MUST exist; inactive panels are present-but-hidden)
    for (const tab of tabs) {
      const controls = tab.getAttribute("aria-controls");
      expect(controls && document.getElementById(controls)).toBeTruthy();
    }
  });

  it("usage_tab_mounts_usage_dashboard", () => {
    renderSettings();
    fireEvent.click(screen.getByRole("tab", { name: /usage/i }));
    // the Usage tab mounts the live dashboard (real projection aggregates)
    expect(screen.getByTestId("usage-table")).toBeTruthy();
    // the credit-pool meter is honestly OMITTED (no daemon source) — the honest note renders instead
    expect(screen.queryByTestId("credit-pool-state")).toBeNull();
    expect(screen.getByTestId("credit-pool-unavailable")).toBeTruthy();
    // the prototype's 14-day spend history has no backing projection — its card
    // is an HONEST pending note, never invented bars (forbidden #2)
    expect(screen.getByTestId("spend-history-pending")).toBeTruthy();
  });

  it("integrations_tab_renders_display_cards_with_disabled_mutations", () => {
    renderSettings();
    fireEvent.click(screen.getByRole("tab", { name: /integrations/i }));
    const panel = screen.getByRole("tabpanel");
    // the prototype card list renders (display fixture — flagged provisional)
    expect(panel.querySelector('[data-integration-id="github"]')).not.toBeNull();
    // Manage/Connect are connector mutations — disabled, not faked (§11.6)
    for (const btn of within(panel as HTMLElement).getAllByRole("button")) {
      expect(btn).toHaveProperty("disabled", true);
    }
    // no fake form controls (forbidden #2)
    expect(panel.querySelector("input, [role='switch'], [role='checkbox']")).toBeNull();
  });

  it("profiles_tab_renders_display_cards_with_disabled_mutations", () => {
    renderSettings();
    fireEvent.click(screen.getByRole("tab", { name: /execution profiles/i }));
    const panel = screen.getByRole("tabpanel");
    expect(panel.querySelector('[data-profile-name="Claude Max Main"]')).not.toBeNull();
    // Add/Configure are 0.5b-gated mutations — disabled, not faked
    for (const btn of within(panel as HTMLElement).getAllByRole("button")) {
      expect(btn).toHaveProperty("disabled", true);
    }
  });

  it("security_tab_renders_policy_ladder_read_only", () => {
    renderSettings();
    fireEvent.click(screen.getByRole("tab", { name: /security/i }));
    const panel = screen.getByRole("tabpanel");
    // the four §5.1 risk rungs render with their risk badges ("Critical" appears
    // as both row name and badge label — getAllByText)
    for (const label of [/read-only & low risk/i, /medium risk/i, /high risk/i, /critical/i]) {
      expect(
        within(panel as HTMLElement).getAllByText(label).length,
      ).toBeGreaterThanOrEqual(1);
    }
    // standing permissions are an honest pending note — no fake toggles
    expect(screen.getByTestId("standing-permissions-pending")).toBeTruthy();
    expect(panel.querySelector("input, [role='switch'], [role='checkbox']")).toBeNull();
  });

  it("tablist_has_exactly_one_tabstop", () => {
    renderSettings();
    const tabs = within(screen.getByRole("tablist")).getAllByRole("tab");
    // the roving invariant: exactly one tab is in the tab order (tabIndex 0), the
    // rest are -1, and the tabstop is the selected tab (§11.6 / APG Tabs).
    const tabstops = tabs.filter((t) => t.tabIndex === 0);
    expect(tabstops).toHaveLength(1);
    expect(tabstops[0]!.getAttribute("aria-selected")).toBe("true");
    for (const nonStop of tabs.filter((t) => t.tabIndex !== 0)) {
      expect(nonStop.tabIndex).toBe(-1);
    }
  });

  it("arrow_moves_focus_and_activates", () => {
    renderSettings();
    // default selected = Integrations (first); focus it, ArrowRight → Profiles
    const integrations = screen.getByRole("tab", { name: /integrations/i });
    integrations.focus();
    fireEvent.keyDown(integrations, { key: "ArrowRight" });
    const profiles = screen.getByRole("tab", { name: /execution profiles/i });
    // automatic activation: focus AND selection move to the next tab + its panel
    expect(document.activeElement).toBe(profiles);
    expect(profiles.getAttribute("aria-selected")).toBe("true");
    expect(
      screen.getByRole("tabpanel").querySelector("[data-profile-name]"),
    ).not.toBeNull();
  });

  it("home_end_jump", () => {
    renderSettings();
    const integrations = screen.getByRole("tab", { name: /integrations/i });
    integrations.focus();
    // End → last tab (Security & policy); focus + select it
    fireEvent.keyDown(integrations, { key: "End" });
    const last = screen.getByRole("tab", { name: /security/i });
    expect(document.activeElement).toBe(last);
    expect(last.getAttribute("aria-selected")).toBe("true");
    // Home → first tab (Integrations); focus + select it
    fireEvent.keyDown(last, { key: "Home" });
    const first = screen.getByRole("tab", { name: /integrations/i });
    expect(document.activeElement).toBe(first);
    expect(first.getAttribute("aria-selected")).toBe("true");
  });
});

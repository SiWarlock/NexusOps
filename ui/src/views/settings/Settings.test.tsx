// @vitest-environment jsdom
import { describe, it, expect, afterEach } from "vitest";
import { cleanup, render, screen, fireEvent, within } from "@testing-library/react";
import { Settings } from "./Settings";
import { usageFixture } from "../../projections/fixtures/proj_usage";

afterEach(cleanup);

const renderSettings = () =>
  render(
    <Settings
      usage={usageFixture.rows}
      creditPool={usageFixture.creditPool ?? null}
    />,
  );

describe("Settings tabbed surface", () => {
  it("renders_tablist_with_aria", () => {
    renderSettings();
    const tablist = screen.getByRole("tablist");
    const tabs = within(tablist).getAllByRole("tab");
    expect(tabs).toHaveLength(5);
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
    // the Usage tab mounts the existing dashboard (the 6.4b relocation)
    expect(screen.getByTestId("usage-table")).toBeTruthy();
  });

  it("pending_tabs_show_honest_empty_state", () => {
    renderSettings();
    for (const name of [/integrations/i, /security/i, /notifications/i]) {
      fireEvent.click(screen.getByRole("tab", { name }));
      const panel = screen.getByRole("tabpanel");
      expect(panel.textContent?.toLowerCase()).toContain("pending");
      // no fabricated data/toggles (forbidden #2) and NOT the usage dashboard
      expect(
        panel.querySelector("input, [role='switch'], [role='checkbox']"),
      ).toBeNull();
      expect(panel.querySelector('[data-testid="usage-table"]')).toBeNull();
    }
  });

  it("execution_profiles_tab_pending_not_bound", () => {
    renderSettings();
    fireEvent.click(screen.getByRole("tab", { name: /execution profiles/i }));
    const panel = screen.getByRole("tabpanel");
    // gated on 0.5b — an honest pending state, no enum binding
    expect(panel.textContent?.toLowerCase()).toContain("pending");
    expect(panel.textContent).toContain("0.5b");
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
    // default selected = Usage (index 3); focus it, ArrowRight → Execution Profiles
    const usage = screen.getByRole("tab", { name: /usage/i });
    usage.focus();
    fireEvent.keyDown(usage, { key: "ArrowRight" });
    const profiles = screen.getByRole("tab", { name: /execution profiles/i });
    // automatic activation: focus AND selection move to the next tab + its panel
    expect(document.activeElement).toBe(profiles);
    expect(profiles.getAttribute("aria-selected")).toBe("true");
    expect(screen.getByRole("tabpanel").textContent).toContain("0.5b");
  });

  it("home_end_jump", () => {
    renderSettings();
    const usage = screen.getByRole("tab", { name: /usage/i });
    usage.focus();
    // End → last tab (Execution Profiles); focus + select it
    fireEvent.keyDown(usage, { key: "End" });
    const last = screen.getByRole("tab", { name: /execution profiles/i });
    expect(document.activeElement).toBe(last);
    expect(last.getAttribute("aria-selected")).toBe("true");
    // Home → first tab (Integrations); focus + select it
    fireEvent.keyDown(last, { key: "Home" });
    const first = screen.getByRole("tab", { name: /integrations/i });
    expect(document.activeElement).toBe(first);
    expect(first.getAttribute("aria-selected")).toBe("true");
  });
});

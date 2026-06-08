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
});

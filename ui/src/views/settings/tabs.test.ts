import { describe, it, expect } from "vitest";
import {
  SETTINGS_TABS,
  DEFAULT_SETTINGS_TAB,
  settingsTabsWithSelection,
} from "./tabs";

describe("settings tabs model", () => {
  it("default_tab_selected", () => {
    const tabs = settingsTabsWithSelection(DEFAULT_SETTINGS_TAB);
    const selected = tabs.filter((t) => t.selected);
    // exactly one tab selected — the sensible default
    expect(selected).toHaveLength(1);
    expect(selected[0]!.key).toBe(DEFAULT_SETTINGS_TAB);
  });

  it("select_tab_switches_active", () => {
    const tabs = settingsTabsWithSelection("security");
    // the chosen tab is selected; every other tab (incl. the default) deselects
    expect(tabs.filter((t) => t.selected).map((t) => t.key)).toEqual(["security"]);
    expect(tabs.find((t) => t.key === DEFAULT_SETTINGS_TAB)?.selected).toBe(false);
  });

  it("execution_profiles_tab_is_gated", () => {
    // the Execution Profiles tab is gated (0.5b) — no frozen-enum binding
    expect(SETTINGS_TABS.find((t) => t.key === "profiles")?.kind).toBe("gated");
    // the five §11.4 tabs are present, in order
    expect(SETTINGS_TABS.map((t) => t.key)).toEqual([
      "integrations",
      "security",
      "notifications",
      "usage",
      "profiles",
    ]);
  });
});

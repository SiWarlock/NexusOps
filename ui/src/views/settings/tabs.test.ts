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

  it("prototype_tab_set_in_order", () => {
    // the prototype's four sections, in its order (kit-views4 SettingsProfiles);
    // the interim Notifications stub was removed at the rebuild (no prototype
    // equivalent — flagged deviation cleanup)
    expect(SETTINGS_TABS.map((t) => t.key)).toEqual([
      "integrations",
      "profiles",
      "usage",
      "security",
    ]);
  });
});

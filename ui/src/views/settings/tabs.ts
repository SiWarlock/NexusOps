// The Settings tab model (§11.2/§11.4): the prototype's four Settings sections
// (kit-views4.jsx SettingsProfiles tab strip — Integrations · Execution profiles
// · Usage · Security & policy). The interim Notifications stub tab was removed
// at the prototype-faithful rebuild (no prototype equivalent; the notifier is
// unbuilt — flagged deviation cleanup, not a feature loss).
//
// Panels carrying daemon-gated data render the prototype's visual treatment
// over PROVISIONAL DISPLAY FIXTURES (display-meta.ts; flagged, never presented
// as live) — Integrations (no connector projection, Phase 7) and Execution
// profiles (ExecutionProfile enum 0.5b-gated). Usage renders REAL projection
// aggregates; Security renders the static policy ladder (read-only — the policy
// engine is daemon Phase 2).

export type SettingsTabKey = "integrations" | "profiles" | "usage" | "security";

export interface SettingsTab {
  key: SettingsTabKey;
  label: string;
}

export const SETTINGS_TABS: SettingsTab[] = [
  { key: "integrations", label: "Integrations" },
  { key: "profiles", label: "Execution profiles" },
  { key: "usage", label: "Usage" },
  { key: "security", label: "Security & policy" },
];

// Default to Integrations — the prototype's first tab.
export const DEFAULT_SETTINGS_TAB: SettingsTabKey = "integrations";

/** Pure derivation: the tabs with a `selected` flag, exactly one true. */
export function settingsTabsWithSelection(
  active: SettingsTabKey,
): (SettingsTab & { selected: boolean })[] {
  return SETTINGS_TABS.map((tab) => ({ ...tab, selected: tab.key === active }));
}

// The Settings tab model (§11.2/§11.4): the five Settings sections + their render
// kind. Daemon-coupled panels render honest "pending" stubs (no fabricated data —
// forbidden #2); the Usage panel mounts the live dashboard (6.4b relocation); the
// Execution Profiles panel is 0.5b-gated (rendered pending, enum NOT bound).

export type SettingsTabKey =
  | "integrations"
  | "security"
  | "notifications"
  | "usage"
  | "profiles";

export interface SettingsTab {
  key: SettingsTabKey;
  label: string;
  /**
   * How the panel renders:
   *  - `usage`   → the live `<UsageDashboard/>`,
   *  - `pending` → an honest empty-state (daemon-coupled surface built ahead of data),
   *  - `gated`   → an honest empty-state, explicitly gated (Execution Profiles / 0.5b).
   */
  kind: "usage" | "pending" | "gated";
  /** The honest note shown for pending/gated panels (never fabricated data). */
  pendingNote?: string;
}

export const SETTINGS_TABS: SettingsTab[] = [
  {
    key: "integrations",
    label: "Integrations",
    kind: "pending",
    pendingNote: "Integrations health — pending integrations (Phase 7).",
  },
  {
    key: "security",
    label: "Security & policy",
    kind: "pending",
    pendingNote: "Security & policy — pending the Action Gateway policy engine (Phase 2).",
  },
  {
    key: "notifications",
    label: "Notifications",
    kind: "pending",
    pendingNote: "Notifications — pending the notifier.",
  },
  { key: "usage", label: "Usage", kind: "usage" },
  {
    key: "profiles",
    label: "Execution Profiles",
    kind: "gated",
    pendingNote: "Execution Profiles — pending the ExecutionProfile enum (0.5b).",
  },
];

// Default to Usage — the content-bearing tab (the relocated dashboard); the other
// tabs are pending stubs, so opening Settings lands on real content.
export const DEFAULT_SETTINGS_TAB: SettingsTabKey = "usage";

/** Pure derivation: the tabs with a `selected` flag, exactly one true. */
export function settingsTabsWithSelection(
  active: SettingsTabKey,
): (SettingsTab & { selected: boolean })[] {
  return SETTINGS_TABS.map((tab) => ({ ...tab, selected: tab.key === active }));
}

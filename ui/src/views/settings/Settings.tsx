import { useState } from "react";
import type { CreditPool, UsageRow } from "../../contracts/index";
import { UsageDashboard } from "../usage/UsageDashboard";
import {
  DEFAULT_SETTINGS_TAB,
  settingsTabsWithSelection,
  type SettingsTab,
  type SettingsTabKey,
} from "./tabs";

interface SettingsProps {
  usage: UsageRow[];
  creditPool: CreditPool | null;
}

function Panel({
  tab,
  usage,
  creditPool,
}: {
  tab: SettingsTab;
  usage: UsageRow[];
  creditPool: CreditPool | null;
}) {
  if (tab.kind === "usage") {
    return <UsageDashboard rows={usage} creditPool={creditPool} />;
  }
  // pending / gated: an honest empty-state — NO fabricated data, NO fake toggles
  // (forbidden #2); the parked intent controls land with the daemon-1.5 seam.
  return (
    <p className="settings__pending" data-testid="settings-pending">
      {tab.pendingNote ?? "Pending."}
    </p>
  );
}

/**
 * The Settings surface (§11.2/§11.4): an ARIA tablist of the five sections. The
 * Usage tab mounts the live `<UsageDashboard/>` (its §11.2 home — relocated from
 * the interim content-view); Integrations/Security/Notifications render honest
 * "pending [Phase X]" stubs; Execution Profiles is 0.5b-gated (no enum binding).
 * Tab controls are `<button>`s (keyboard-reachable via Tab + the global focus
 * ring — Lesson §9); arrow-key roving is a later a11y enhancement. Reads only
 * real state (forbidden #2). Panels are unstyled until the 6.5 theme pass.
 */
export function Settings({ usage, creditPool }: SettingsProps) {
  const [active, setActive] = useState<SettingsTabKey>(DEFAULT_SETTINGS_TAB);
  const tabs = settingsTabsWithSelection(active);

  return (
    <div className="settings" aria-label="Settings">
      <div role="tablist" aria-label="Settings sections" className="settings__tablist">
        {tabs.map((tab) => (
          <button
            key={tab.key}
            type="button"
            role="tab"
            id={`settings-tab-${tab.key}`}
            aria-selected={tab.selected}
            aria-controls={`settings-panel-${tab.key}`}
            onClick={() => setActive(tab.key)}
          >
            {tab.label}
          </button>
        ))}
      </div>
      {/* All five tabpanels stay in the DOM so every tab's aria-controls resolves
          (WAI-ARIA); inactive panels are `hidden` (and excluded from the a11y
          tree). tabIndex=0 lets a keyboard user Tab into a panel whose content
          isn't itself focusable. Content renders only for the active tab. */}
      {tabs.map((tab) => (
        <div
          key={tab.key}
          role="tabpanel"
          id={`settings-panel-${tab.key}`}
          aria-labelledby={`settings-tab-${tab.key}`}
          className="settings__panel"
          hidden={!tab.selected}
          tabIndex={0}
        >
          {tab.selected ? (
            <Panel tab={tab} usage={usage} creditPool={creditPool} />
          ) : null}
        </div>
      ))}
    </div>
  );
}

import { useRef, useState, type CSSProperties, type KeyboardEvent, type ReactNode } from "react";
import {
  Brain,
  Cpu,
  GitCompareArrows,
  Info,
  Plug,
  Plus,
  RefreshCw,
  Settings as SettingsIcon,
  Settings2,
  Shield,
  ShieldAlert,
  ShieldCheck,
  ShieldX,
  SquareKanban,
} from "lucide-react";

// lucide-react 1.x removed brand icons — the GitHub mark is inlined with the
// same stroke geometry the prototype's lucide UMD (0.460) renders.
function GithubMark({ size = 17 }: { size?: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="M15 22v-4a4.8 4.8 0 0 0-1-3.5c3 0 6-2 6-5.5.08-1.25-.27-2.48-1-3.5.28-1.15.28-2.35 0-3.5 0 0-1 0-3 1.5-2.64-.5-5.36-.5-8 0C6 2 5 2 5 2c-.3 1.15-.3 2.35 0 3.5A5.403 5.403 0 0 0 4 9c0 3.5 3 5.5 6 5.5-.39.49-.68 1.05-.85 1.65-.17.6-.22 1.23-.15 1.85v4" />
      <path d="M9 18c-4.51 2-5-2-7-2" />
    </svg>
  );
}
import type { UsageRow } from "../../contracts/index";
import {
  Badge,
  Button,
  DisplayStatusPill,
  IconButton,
  RiskBadge,
  UsageMeter,
} from "../../design-system/kit";
import {
  integrationsDisplayFixture,
  profilesDisplayFixture,
  type ProfileHealth,
  type RiskLevel,
} from "../../shell/display-meta";
import { UsageDashboard } from "../usage/UsageDashboard";
import {
  DEFAULT_SETTINGS_TAB,
  settingsTabsWithSelection,
  type SettingsTabKey,
} from "./tabs";
import { isRovingKey, nextTabIndex } from "../../a11y/roving";

interface SettingsProps {
  usage: UsageRow[];
}

function Eyebrow({ children, style }: { children: ReactNode; style?: CSSProperties }) {
  return (
    <div
      style={{
        font: "var(--fw-semibold) var(--fs-micro)/1 var(--font-sans)",
        letterSpacing: "var(--tracking-caps)",
        textTransform: "uppercase",
        color: "var(--text-faint)",
        ...style,
      }}
    >
      {children}
    </div>
  );
}

const INTEGRATION_ICON: Record<string, ReactNode> = {
  cpu: <Cpu size={17} />,
  github: <GithubMark size={17} />,
  "square-kanban": <SquareKanban size={17} />,
  brain: <Brain size={17} />,
};

/** Integrations panel (prototype card list — PROVISIONAL DISPLAY FIXTURE; no
 *  connector projection yet, Phase 7. Manage/Connect are mutations — disabled). */
function IntegrationsPanel() {
  return (
    <div style={{ padding: 16, maxWidth: 760, display: "flex", flexDirection: "column", gap: 10 }}>
      {integrationsDisplayFixture.map((it) => (
        <div
          key={it.id}
          data-integration-id={it.id}
          style={{
            border: `1px solid ${it.connected ? "var(--border-default)" : "var(--caution-line)"}`,
            borderRadius: "var(--r-3)",
            background: it.connected ? "var(--surface-card)" : "var(--caution-surface)",
            padding: "13px 14px",
            display: "flex",
            alignItems: "center",
            gap: 13,
          }}
        >
          <span
            aria-hidden="true"
            style={{
              width: 34,
              height: 34,
              flex: "none",
              borderRadius: "var(--r-2)",
              background: "var(--surface-active)",
              color: it.connected ? "var(--text-secondary)" : "var(--caution-ink)",
              display: "inline-flex",
              alignItems: "center",
              justifyContent: "center",
            }}
          >
            {INTEGRATION_ICON[it.icon]}
          </span>
          <div style={{ flex: 1, minWidth: 0 }}>
            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
              <span style={{ font: "var(--fw-semibold) var(--fs-body) var(--font-sans)" }}>
                {it.name}
              </span>
              <DisplayStatusPill
                status={it.connected ? "completed" : "idle"}
                size="xs"
                label={it.connected ? "Connected" : "Not connected"}
              />
              {it.scope ? (
                <Badge mono style={{ color: "var(--text-faint)" }}>
                  {it.scope}
                </Badge>
              ) : null}
            </div>
            <div style={{ font: "var(--fs-meta) var(--font-sans)", color: "var(--text-muted)", marginTop: 3 }}>
              {it.detail}
            </div>
          </div>
          <span title="Integration management arrives with the connector contracts (Phase 7)">
            <Button
              variant="secondary"
              size="sm"
              icon={it.connected ? <Settings2 size={13} /> : <Plug size={13} />}
              disabled
            >
              {it.action}
            </Button>
          </span>
        </div>
      ))}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 9,
          padding: "11px 13px",
          borderRadius: "var(--r-3)",
          border: "1px dashed var(--border-default)",
          color: "var(--text-muted)",
          font: "var(--fs-meta)/1.5 var(--font-sans)",
        }}
      >
        <GitCompareArrows size={15} aria-hidden="true" style={{ color: "var(--text-faint)", flex: "none" }} />
        <span>
          Linking <strong style={{ color: "var(--text-secondary)" }}>GitHub ↔ Linear</strong> lets a
          Linear ticket and its GitHub issue/PR resolve to one task in the inbox. Connect Linear to
          enable bi-directional mapping.
        </span>
      </div>
    </div>
  );
}

// Profile health → kit pill kind + label (display-only states; no frozen machine).
const HEALTH: Record<ProfileHealth, { pill: "completed" | "idle" | "stale" | "failed"; label: string }> = {
  active: { pill: "completed", label: "Active" },
  available: { pill: "idle", label: "Available" },
  "rate-limited": { pill: "stale", label: "Rate-limited" },
  "auth-expired": { pill: "failed", label: "Auth expired" },
};

/** Execution profiles panel (prototype cards — PROVISIONAL DISPLAY FIXTURE;
 *  ExecutionProfile contract is 0.5b-gated. Add/Re-auth/Configure disabled). */
function ProfilesPanel() {
  return (
    <div style={{ padding: 16, maxWidth: 800, display: "flex", flexDirection: "column", gap: 10 }}>
      <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
        <Badge mono style={{ color: "var(--text-muted)" }}>
          {profilesDisplayFixture.length} configured
        </Badge>
        <div style={{ marginLeft: "auto" }}>
          <span title="Profile management arrives with the ExecutionProfile contract (0.5b)">
            <Button variant="primary" size="sm" icon={<Plus size={14} />} disabled>
              Add profile
            </Button>
          </span>
        </div>
      </div>
      {profilesDisplayFixture.map((p) => {
        const h = HEALTH[p.health];
        const over = p.usage >= 95;
        return (
          <div
            key={p.name}
            data-profile-name={p.name}
            style={{
              border: `1px solid ${over ? "var(--warning-line)" : "var(--border-default)"}`,
              borderRadius: "var(--r-3)",
              background: "var(--surface-card)",
              padding: "13px 14px",
              display: "grid",
              gridTemplateColumns: "auto 1fr 220px auto",
              alignItems: "center",
              gap: 14,
            }}
          >
            <span
              aria-hidden="true"
              style={{
                width: 34,
                height: 34,
                flex: "none",
                borderRadius: "var(--r-2)",
                background:
                  p.provider === "claude"
                    ? "var(--domain-claude-surface)"
                    : "var(--domain-codex-surface)",
                display: "inline-flex",
                alignItems: "center",
                justifyContent: "center",
                font: "var(--font-mono)",
                color: p.provider === "claude" ? "var(--domain-claude)" : "var(--domain-codex)",
                fontSize: 16,
              }}
            >
              {p.provider === "claude" ? "✻" : "⌁"}
            </span>
            <div style={{ minWidth: 0 }}>
              <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <span style={{ font: "var(--fw-semibold) var(--fs-body) var(--font-sans)" }}>
                  {p.name}
                </span>
                <DisplayStatusPill status={h.pill} size="xs" label={h.label} />
              </div>
              <div style={{ font: "var(--fs-meta) var(--font-sans)", color: "var(--text-muted)", marginTop: 3 }}>
                {p.note} · {p.sessions} active session{p.sessions === 1 ? "" : "s"}
              </div>
            </div>
            <div>
              <UsageMeter
                label="Quota"
                value={p.usage}
                max={p.limit}
                valueText={`${p.usage} / ${p.limit}${p.resets !== "—" ? " · resets " + p.resets : ""}`}
              />
            </div>
            <div style={{ display: "flex", gap: 6, justifyContent: "flex-end" }}>
              {p.health === "auth-expired" ? (
                <span title="Re-auth arrives with the profile/keychain contract (0.5b)">
                  <Button variant="secondary" size="sm" icon={<RefreshCw size={13} />} disabled>
                    Re-auth
                  </Button>
                </span>
              ) : (
                <IconButton label={`Configure ${p.name}`} size="sm" disabled>
                  <Settings2 size={15} />
                </IconButton>
              )}
            </div>
          </div>
        );
      })}
    </div>
  );
}

// The §5.1 risk ladder (static policy reference — the editable policy engine is
// daemon Phase 2; rows are read-only with disabled Edit affordances).
const POLICY_LADDER: { icon: ReactNode; name: string; desc: string; level: RiskLevel }[] = [
  {
    icon: <ShieldCheck size={16} />,
    name: "Read-only & low risk",
    desc: "Auto-approved — no confirmation",
    level: "readonly",
  },
  {
    icon: <Shield size={16} />,
    name: "Medium risk",
    desc: "Confirm each action (create worktree, send to agent, draft PR)",
    level: "medium",
  },
  {
    icon: <ShieldAlert size={16} />,
    name: "High risk",
    desc: "Explicit confirmation (git writes, run commands, ticket changes)",
    level: "high",
  },
  {
    icon: <ShieldX size={16} />,
    name: "Critical",
    desc: "Typed confirmation · no automation (force-push, delete, secrets)",
    level: "critical",
  },
];

/** Security & policy panel: the approval-policy ladder + standing permissions
 *  (read-only — the policy engine + standing-permission store are daemon
 *  contracts; toggles render as disabled state, never fake switches). */
function SecurityPanel() {
  return (
    <div style={{ padding: 16, maxWidth: 760, display: "flex", flexDirection: "column", gap: 16 }}>
      <div>
        <Eyebrow style={{ marginBottom: 10 }}>Default approval policy</Eyebrow>
        <div style={{ display: "flex", flexDirection: "column", gap: 7 }}>
          {POLICY_LADDER.map((row) => (
            <div
              key={row.level}
              style={{
                display: "flex",
                alignItems: "center",
                gap: 11,
                padding: "11px 12px",
                borderRadius: "var(--r-2)",
                border: "1px solid var(--border-default)",
                background: "var(--surface-card)",
              }}
            >
              <span aria-hidden="true" style={{ color: `var(--risk-${row.level})`, display: "inline-flex" }}>
                {row.icon}
              </span>
              <div style={{ flex: 1, minWidth: 0 }}>
                <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                  <span style={{ font: "var(--fw-medium) var(--fs-body) var(--font-sans)" }}>
                    {row.name}
                  </span>
                  <RiskBadge level={row.level} />
                </div>
                <div style={{ font: "var(--fs-meta) var(--font-sans)", color: "var(--text-muted)", marginTop: 2 }}>
                  {row.desc}
                </div>
              </div>
              <IconButton label={`Edit ${row.name} policy`} size="sm" disabled>
                <Settings2 size={15} />
              </IconButton>
            </div>
          ))}
        </div>
      </div>
      <div>
        <Eyebrow style={{ marginBottom: 10 }}>Standing permissions (policy automation)</Eyebrow>
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: 9,
            padding: "11px 13px",
            borderRadius: "var(--r-3)",
            border: "1px dashed var(--border-default)",
            color: "var(--text-muted)",
            font: "var(--fs-meta)/1.5 var(--font-sans)",
          }}
          data-testid="standing-permissions-pending"
        >
          <Info size={13} aria-hidden="true" style={{ flex: "none" }} />
          <span>
            Standing permissions are opt-in, narrow, revocable, and fully audited via the Action
            Gateway. They arrive with the policy engine (daemon Phase 2) — nothing to configure yet.
          </span>
        </div>
      </div>
    </div>
  );
}

/**
 * The Settings surface (ported from kit-views4.jsx SettingsProfiles): header +
 * the prototype's four underline tabs (Integrations · Execution profiles ·
 * Usage · Security & policy). Tab controls are `<button>`s with WAI-ARIA roving
 * tabindex (exactly one tabstop; Arrow/Home/End move focus + automatically
 * activate — APG Tabs, Lesson §9). Usage renders REAL projection aggregates;
 * Integrations/Profiles render the prototype treatment over provisional display
 * fixtures (flagged); every mutation affordance is disabled, not faked (§11.6).
 */
export function Settings({ usage }: SettingsProps) {
  const [active, setActive] = useState<SettingsTabKey>(DEFAULT_SETTINGS_TAB);
  const tabs = settingsTabsWithSelection(active);
  const tabRefs = useRef<(HTMLButtonElement | null)[]>([]);
  const activeIndex = tabs.findIndex((t) => t.selected);

  // Roving + automatic activation: Arrow/Home/End move focus to the computed tab
  // AND select it (panels are instant, so activation has no latency — APG).
  // The current index is read from the focused tab at event time (not the
  // render-time `activeIndex`), so rapid repeats never compute off a stale
  // closure; `activeIndex` is the fallback if focus isn't on a tab.
  // preventDefault stops the keys also scrolling the surface.
  const onTablistKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    if (!isRovingKey(e.key)) return;
    e.preventDefault();
    const focused = tabRefs.current.findIndex((el) => el === document.activeElement);
    const current = focused >= 0 ? focused : activeIndex;
    const next = nextTabIndex(current, tabs.length, e.key);
    if (next === current) return; // already at the target (e.g. Home on the first tab)
    setActive(tabs[next]!.key);
    tabRefs.current[next]?.focus();
  };

  return (
    <div
      className="settings"
      aria-label="Settings"
      style={{ height: "100%", overflowY: "auto", background: "var(--surface-canvas)" }}
    >
      <div style={{ padding: "14px 16px 0", borderBottom: "1px solid var(--border-subtle)" }}>
        <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 12 }}>
          <span aria-hidden="true" style={{ color: "var(--text-secondary)", display: "inline-flex" }}>
            <SettingsIcon size={18} />
          </span>
          <h1 style={{ margin: 0, font: "var(--fw-semibold) var(--fs-h3)/1 var(--font-sans)" }}>
            Settings
          </h1>
        </div>
        <div
          role="tablist"
          aria-label="Settings sections"
          className="settings__tablist"
          onKeyDown={onTablistKeyDown}
          style={{ display: "flex", gap: 2 }}
        >
          {tabs.map((tab, i) => (
            <button
              key={tab.key}
              ref={(el) => {
                tabRefs.current[i] = el;
              }}
              type="button"
              role="tab"
              id={`settings-tab-${tab.key}`}
              aria-selected={tab.selected}
              aria-controls={`settings-panel-${tab.key}`}
              tabIndex={tab.selected ? 0 : -1}
              onClick={() => setActive(tab.key)}
              style={{
                padding: "8px 12px",
                border: "none",
                background: "transparent",
                cursor: "pointer",
                font: `${tab.selected ? "var(--fw-semibold)" : "var(--fw-medium)"} var(--fs-label) var(--font-sans)`,
                color: tab.selected ? "var(--accent-ink)" : "var(--text-muted)",
                boxShadow: tab.selected ? "inset 0 -2px 0 var(--accent-solid)" : "none",
              }}
            >
              {tab.label}
            </button>
          ))}
        </div>
      </div>
      {/* All tabpanels stay in the DOM so every tab's aria-controls resolves
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
            tab.key === "usage" ? (
              <UsageDashboard rows={usage} />
            ) : tab.key === "integrations" ? (
              <IntegrationsPanel />
            ) : tab.key === "profiles" ? (
              <ProfilesPanel />
            ) : (
              <SecurityPanel />
            )
          ) : null}
        </div>
      ))}
    </div>
  );
}

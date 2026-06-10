import {
  ArrowLeft,
  ArrowRight,
  Bell,
  Brain,
  GitPullRequest,
  Inbox,
  Search,
  Settings,
  ShieldCheck,
} from "lucide-react";
import type { CSSProperties, ReactNode } from "react";
import { Button, IconButton } from "../design-system/kit";
import type { ProjectActivityRow } from "../contracts/index";
import type { ConnectionState } from "../connection/state";
import type { ProjectSwitcherCounts } from "./derive";
import { projectDisplayFixture } from "./display-meta";
import { useActiveProject } from "./active-project";
import { ProjectSwitcher } from "./ProjectSwitcher";

const ZERO: ProjectSwitcherCounts = {
  activeSessions: 0,
  openPRs: 0,
  waitingOnYou: 0,
};

// Icon-only nav button (ported from the prototype's hand-rolled back/forward
// chrome). The accessible NAME is a visually-hidden child (Lesson §6 + §11.7).
function NavIconButton({
  label,
  disabled,
  onClick,
  children,
}: {
  label: string;
  disabled: boolean;
  onClick: () => void;
  children: ReactNode;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      disabled={disabled}
      style={{
        display: "inline-flex",
        alignItems: "center",
        justifyContent: "center",
        width: 26,
        height: 26,
        borderRadius: "var(--r-2)",
        border: "none",
        background: "transparent",
        cursor: disabled ? "default" : "pointer",
        color: disabled ? "var(--text-faint)" : "var(--text-secondary)",
        opacity: disabled ? 0.4 : 1,
      }}
    >
      <span aria-hidden="true" style={{ display: "inline-flex" }}>
        {children}
      </span>
      <span className="sr-only">{label}</span>
    </button>
  );
}

// The local-runtime health pill. Driven by the REAL transport connection state
// (the §11.4 daemon-connection signal — glyph dot + text label, never color
// alone). The prototype's static "runtime" pill, made live.
function RuntimePill({ connection }: { connection: ConnectionState }) {
  const tone =
    connection === "connected"
      ? { line: "var(--success-line)", surface: "var(--success-surface)", ink: "var(--success-ink)", solid: "var(--success-solid)" }
      : connection === "disconnected"
        ? { line: "var(--danger-line)", surface: "var(--danger-surface)", ink: "var(--danger-ink)", solid: "var(--danger-solid)" }
        : { line: "var(--caution-line)", surface: "var(--caution-surface)", ink: "var(--caution-ink)", solid: "var(--caution-solid)" };
  const label = connection === "connected" ? "runtime" : connection;
  return (
    <span
      title={`Local runtime: ${connection}`}
      data-state={connection}
      style={{
        display: "inline-flex",
        alignItems: "center",
        gap: 5,
        padding: "0 8px",
        height: 24,
        borderRadius: 999,
        border: `1px solid ${tone.line}`,
        background: tone.surface,
        font: "10px var(--font-mono)",
        color: tone.ink,
      }}
    >
      <span
        aria-hidden="true"
        style={{ width: 6, height: 6, borderRadius: 999, background: tone.solid }}
      />
      {label}
    </span>
  );
}

const metaMono: CSSProperties = {
  font: "var(--fs-meta) var(--font-mono)",
  color: "var(--text-faint)",
};

/**
 * Top bar (ported from kit-shell.jsx TopBar): traffic lights, back/forward
 * history nav, the project switcher + repo slug + live counters, the ⌘K command
 * field, and the right cluster (runtime pill, Task Inbox, Human Input queue,
 * Action Gateway, Brain, Settings). Counters are derived from projections —
 * never invented. Overlay triggers (palette/inbox/queue/gateway/brain) are
 * DISABLED until their overlay surfaces land (wire-or-disable §11.6 — flagged,
 * not faked).
 */
export function TopBar({
  projects,
  counts,
  connection = "connected",
  waiting = 0,
  onOpenSettings,
  onOpenBrain,
  onOpenPalette,
  onOpenTasks,
  onOpenHiq,
  onOpenGateway,
  onBack,
  onForward,
  canBack,
  canForward,
}: {
  projects: ProjectActivityRow[];
  counts: Record<string, ProjectSwitcherCounts>;
  connection?: ConnectionState;
  /** Global waiting-on-you count (HIQ badge). */
  waiting?: number;
  onOpenSettings: () => void;
  /** Opens the Brain drawer. */
  onOpenBrain?: () => void;
  /** Opens the ⌘K command palette. */
  onOpenPalette?: () => void;
  /** Opens the Task Inbox drawer. */
  onOpenTasks?: () => void;
  /** Opens the Human Input queue drawer. */
  onOpenHiq?: () => void;
  /** Opens the Gateway modal on the first pending approval (absent → disabled). */
  onOpenGateway?: (() => void) | undefined;
  onBack: () => void;
  onForward: () => void;
  canBack: boolean;
  canForward: boolean;
}) {
  const { activeProjectId } = useActiveProject();
  const active = counts[activeProjectId ?? ""] ?? ZERO;
  const repo = projectDisplayFixture[activeProjectId ?? ""]?.repo;

  return (
    <header className="topbar">
      {/* traffic lights (window chrome) */}
      <div style={{ display: "flex", gap: 6, alignItems: "center", paddingRight: 2 }} aria-hidden="true">
        {[0, 1, 2].map((i) => (
          <span
            key={i}
            style={{
              width: 11,
              height: 11,
              borderRadius: 999,
              background: "#3b3b40",
              border: "1px solid var(--border-strong)",
            }}
          />
        ))}
      </div>
      <nav aria-label="History" style={{ display: "flex", gap: 1 }}>
        <NavIconButton label="Back" disabled={!canBack} onClick={onBack}>
          <ArrowLeft size={16} />
        </NavIconButton>
        <NavIconButton label="Forward" disabled={!canForward} onClick={onForward}>
          <ArrowRight size={16} />
        </NavIconButton>
      </nav>
      <span style={{ width: 1, height: 18, background: "var(--border-default)" }} aria-hidden="true" />
      <ProjectSwitcher projects={projects} counts={counts} />
      {repo ? <span style={metaMono}>{repo}</span> : null}
      {/* live counters: active sessions · open PRs · waiting-on-you */}
      <span
        style={{
          display: "inline-flex",
          alignItems: "center",
          gap: 11,
          marginLeft: 2,
          font: "var(--fs-meta) var(--font-mono)",
          color: "var(--text-muted)",
        }}
      >
        <span title="active sessions" style={{ display: "inline-flex", alignItems: "center", gap: 4 }}>
          <span
            aria-hidden="true"
            style={{ width: 6, height: 6, borderRadius: 999, background: "var(--live-solid)" }}
          />
          {active.activeSessions}
        </span>
        <span title="open PRs" style={{ display: "inline-flex", alignItems: "center", gap: 4 }}>
          <GitPullRequest size={12} aria-hidden="true" />
          {active.openPRs}
        </span>
        {active.waitingOnYou > 0 ? (
          <span
            title="waiting on you"
            style={{ display: "inline-flex", alignItems: "center", gap: 4, color: "var(--attention-ink)" }}
          >
            ◆ {active.waitingOnYou}
          </span>
        ) : null}
      </span>
      {/* ⌘K command field — opens the command palette. */}
      <button
        type="button"
        disabled={!onOpenPalette}
        onClick={onOpenPalette}
        title="Search or run a command (⌘K)"
        style={{
          marginLeft: "auto",
          width: 320,
          display: "flex",
          alignItems: "center",
          gap: 8,
          height: 28,
          padding: "0 10px",
          borderRadius: "var(--r-2)",
          background: "var(--surface-input)",
          border: "1px solid var(--border-default)",
          color: "var(--text-muted)",
          cursor: "text",
          font: "var(--fs-label) var(--font-sans)",
        }}
      >
        <Search size={14} aria-hidden="true" /> Search or run a command…
        <kbd
          style={{
            marginLeft: "auto",
            padding: "1px 5px",
            borderRadius: 3,
            background: "var(--surface-active)",
            border: "1px solid var(--border-default)",
            font: "10px var(--font-mono)",
          }}
        >
          ⌘K
        </kbd>
      </button>
      <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
        <RuntimePill connection={connection} />
        <IconButton label="Task Inbox" disabled={!onOpenTasks} onClick={onOpenTasks}>
          <Inbox size={16} />
        </IconButton>
        <IconButton
          label="Human input queue"
          badge={waiting > 0 ? waiting : undefined}
          disabled={!onOpenHiq}
          onClick={onOpenHiq}
        >
          <Bell size={16} />
        </IconButton>
        <IconButton label="Action Gateway" disabled={!onOpenGateway} onClick={onOpenGateway}>
          <ShieldCheck size={16} />
        </IconButton>
        <Button
          variant="brain"
          size="sm"
          icon={<Brain size={16} />}
          disabled={!onOpenBrain}
          onClick={onOpenBrain}
        >
          Brain
        </Button>
        <IconButton label="Settings" onClick={onOpenSettings}>
          <Settings size={16} />
        </IconButton>
      </div>
    </header>
  );
}

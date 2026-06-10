import { useMemo, useState, type ReactNode } from "react";
import {
  Brain,
  CodeXml,
  GitPullRequest,
  Inbox,
  LayoutDashboard,
  LayoutGrid,
  ListChecks,
  Package,
  ScrollText,
  Search,
  Settings,
  ShieldCheck,
  Terminal,
  UsersRound,
  Workflow,
} from "lucide-react";
import type { ViewName } from "../shell/view-history";
import { Overlay } from "./Overlay";

export type PaletteAction =
  | { kind: "view"; view: ViewName }
  | { kind: "overlay"; overlay: "brain" | "tasks" | "hiq" | "gateway" };

interface Command {
  icon: ReactNode;
  label: string;
  hint: string;
  action: PaletteAction;
}

// The command set (ported from kit-overlays.jsx CMDS) — every entry routes to a
// REAL surface (view nav or a built overlay); nothing fires a mutation.
const COMMANDS: Command[] = [
  { icon: <Brain size={15} />, label: "Ask Project Brain", hint: "co-pilot", action: { kind: "overlay", overlay: "brain" } },
  { icon: <Inbox size={15} />, label: "Open Task Inbox", hint: "GitHub · Linear", action: { kind: "overlay", overlay: "tasks" } },
  { icon: <ShieldCheck size={15} />, label: "Open Human Input queue", hint: "approvals", action: { kind: "overlay", overlay: "hiq" } },
  { icon: <LayoutDashboard size={15} />, label: "Open Command Center", hint: "triage", action: { kind: "view", view: "command" } },
  { icon: <LayoutGrid size={15} />, label: "Open Projects overview", hint: "projects", action: { kind: "view", view: "projects" } },
  { icon: <Workflow size={15} />, label: "Open Project Graph", hint: "observability", action: { kind: "view", view: "graph" } },
  { icon: <ListChecks size={15} />, label: "Open Implementation Plan", hint: "phases", action: { kind: "view", view: "plan" } },
  { icon: <CodeXml size={15} />, label: "Open Editor", hint: "IDE", action: { kind: "view", view: "editor" } },
  { icon: <Terminal size={15} />, label: "Open Session Terminal", hint: "sessions", action: { kind: "view", view: "terminal" } },
  { icon: <GitPullRequest size={15} />, label: "Review pull requests", hint: "PRs", action: { kind: "view", view: "code" } },
  { icon: <UsersRound size={15} />, label: "Open Agent Team", hint: "cc-crew", action: { kind: "view", view: "team" } },
  { icon: <Package size={15} />, label: "Open Workflow Packs", hint: "/team-start", action: { kind: "view", view: "packs" } },
  { icon: <ScrollText size={15} />, label: "Open Audit Trail", hint: "events", action: { kind: "view", view: "audit" } },
  { icon: <Settings size={15} />, label: "Execution profiles & settings", hint: "config", action: { kind: "view", view: "settings" } },
];

/**
 * Command palette (⌘K — ported from kit-overlays.jsx CommandPalette): a
 * filterable command list over the cockpit's REAL navigation + overlay
 * surfaces. Enter runs the highlighted command; ArrowUp/Down move the
 * highlight; the filter is live local state.
 */
export function CommandPalette({
  onClose,
  onAction,
}: {
  onClose: () => void;
  onAction: (a: PaletteAction) => void;
}) {
  const [query, setQuery] = useState("");
  const [cursor, setCursor] = useState(0);
  const matches = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return COMMANDS;
    return COMMANDS.filter(
      (c) => c.label.toLowerCase().includes(q) || c.hint.toLowerCase().includes(q),
    );
  }, [query]);
  const active = Math.min(cursor, Math.max(0, matches.length - 1));

  const run = (c: Command | undefined) => {
    if (!c) return;
    onClose();
    onAction(c.action);
  };

  return (
    <Overlay onClose={onClose} align="top" width={560} label="Command palette">
      <div
        style={{
          background: "var(--surface-card)",
          border: "1px solid var(--border-strong)",
          borderRadius: "var(--r-4)",
          boxShadow: "var(--elev-4)",
          overflow: "hidden",
        }}
      >
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: 10,
            padding: "12px 14px",
            borderBottom: "1px solid var(--border-default)",
          }}
        >
          <Search size={16} aria-hidden="true" style={{ color: "var(--text-faint)" }} />
          <input
            data-autofocus
            value={query}
            onChange={(e) => {
              setQuery(e.target.value);
              setCursor(0);
            }}
            onKeyDown={(e) => {
              if (e.key === "ArrowDown") {
                e.preventDefault();
                setCursor((c) => Math.min(c + 1, matches.length - 1));
              } else if (e.key === "ArrowUp") {
                e.preventDefault();
                setCursor((c) => Math.max(c - 1, 0));
              } else if (e.key === "Enter") {
                e.preventDefault();
                run(matches[active]);
              }
            }}
            placeholder="Search objects or run a command…"
            aria-label="Search commands"
            style={{
              flex: 1,
              background: "transparent",
              border: "none",
              outline: "none",
              color: "var(--text-primary)",
              font: "var(--fs-body-lg) var(--font-sans)",
            }}
          />
          <kbd
            style={{
              font: "10px var(--font-mono)",
              color: "var(--text-faint)",
              border: "1px solid var(--border-default)",
              borderRadius: 3,
              padding: "1px 5px",
            }}
          >
            esc
          </kbd>
        </div>
        <div style={{ padding: 6, maxHeight: 320, overflowY: "auto" }} data-testid="palette-commands">
          <div
            style={{
              font: "var(--fs-micro) var(--font-sans)",
              letterSpacing: "var(--tracking-caps)",
              textTransform: "uppercase",
              color: "var(--text-faint)",
              padding: "7px 9px 4px",
            }}
          >
            Commands
          </div>
          {matches.map((c, i) => (
            <button
              key={c.label}
              type="button"
              onClick={() => run(c)}
              onMouseEnter={() => setCursor(i)}
              style={{
                display: "flex",
                alignItems: "center",
                gap: 10,
                width: "100%",
                padding: "8px 9px",
                borderRadius: "var(--r-2)",
                border: "none",
                cursor: "pointer",
                textAlign: "left",
                background: i === active ? "var(--accent-surface)" : "transparent",
              }}
            >
              <span
                aria-hidden="true"
                style={{ display: "inline-flex", color: i === active ? "var(--accent-ink)" : "var(--text-muted)" }}
              >
                {c.icon}
              </span>
              <span style={{ flex: 1, font: "var(--fs-body) var(--font-sans)", color: "var(--text-primary)" }}>
                {c.label}
              </span>
              <span style={{ font: "var(--fs-meta) var(--font-mono)", color: "var(--text-faint)" }}>{c.hint}</span>
            </button>
          ))}
          {matches.length === 0 ? (
            <div style={{ font: "var(--fs-meta) var(--font-sans)", color: "var(--text-faint)", padding: "10px 9px" }}>
              No matching command.
            </div>
          ) : null}
        </div>
      </div>
    </Overlay>
  );
}

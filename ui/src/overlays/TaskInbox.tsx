import { useState, type ReactNode } from "react";
import { CircleDot, Inbox, ListChecks, RefreshCw, SquareKanban, X } from "lucide-react";
import { Badge, IconButton } from "../design-system/kit";
import { Overlay } from "./Overlay";

// ── Task intake display fixture ──────────────────────────────────────────────
// No Task/intake projection exists yet (Phase 7 connectors) — PROVISIONAL
// DISPLAY FIXTURE (flagged); dispatch is the intent seam (disabled).
interface TaskFx {
  source: "linear" | "github" | "plan";
  id: string;
  title: string;
  priority: "Urgent" | "High" | "Medium" | "Low";
  labels: string[];
  status: string;
  planTask?: string;
  session?: boolean;
}

const tasksFixture: TaskFx[] = [
  { source: "linear", id: "ENG-310", title: "Refactor auth module", priority: "High", labels: ["auth", "backend"], status: "In progress", planTask: "Phase 2.1", session: true },
  { source: "github", id: "#214", title: "Fix flaky integration test", priority: "Urgent", labels: ["bug", "ci"], status: "In progress", session: true },
  { source: "plan", id: "Phase 2.3", title: "Project observability graph", priority: "High", labels: ["phase-2"], status: "Ready" },
  { source: "plan", id: "Phase 3.1", title: "Action Gateway approval cards", priority: "Medium", labels: ["phase-3", "gateway"], status: "Backlog" },
  { source: "linear", id: "ENG-240", title: "Token usage meter accuracy", priority: "Medium", labels: ["usage"], status: "Todo", planTask: "Phase 3.4" },
  { source: "github", id: "#190", title: "Workflow pack registry schema", priority: "Low", labels: ["workflow"], status: "Todo" },
  { source: "linear", id: "ENG-205", title: "Audit trail export to NDJSON", priority: "Low", labels: ["audit"], status: "Done" },
];

const SOURCE: Record<TaskFx["source"], { icon: ReactNode; tone: string; surf: string }> = {
  linear: { icon: <SquareKanban size={11} />, tone: "var(--domain-linear)", surf: "var(--domain-linear-surface)" },
  github: { icon: <CircleDot size={11} />, tone: "var(--slate-ink)", surf: "var(--slate-surface)" },
  plan: { icon: <ListChecks size={11} />, tone: "var(--brain-ink)", surf: "var(--brain-surface)" },
};
const PRIO: Record<TaskFx["priority"], string> = {
  Urgent: "var(--danger-ink)",
  High: "var(--attention-ink)",
  Medium: "var(--caution-ink)",
  Low: "var(--text-muted)",
};
const TABS = ["All", "Linear", "GitHub", "Plan tasks", "Completed"] as const;

/**
 * The Task Inbox drawer (⌘⇧P — ported from kit-tasks.jsx TaskInbox): task
 * intake across GitHub issues · Linear tickets · plan tasks, with source tabs.
 * DISPLAY-ONLY over a provisional fixture (no Task projection — Phase 7
 * connectors; flagged). Task chips are inert (dispatch + drag-to-session are
 * the intent seam — §11.6); Sync is a connector mutation (disabled).
 */
export function TaskInbox({ onClose }: { onClose: () => void }) {
  const [tab, setTab] = useState<(typeof TABS)[number]>("All");
  const tasks = tasksFixture.filter((t) => {
    if (tab === "All") return t.status !== "Done";
    if (tab === "Linear") return t.source === "linear";
    if (tab === "GitHub") return t.source === "github";
    if (tab === "Plan tasks") return t.source === "plan";
    return t.status === "Done";
  });
  return (
    <Overlay onClose={onClose} align="right" width={400} label="Task Inbox">
      <div
        style={{
          width: "100%",
          height: "100%",
          background: "var(--surface-panel)",
          borderLeft: "1px solid var(--border-strong)",
          boxShadow: "var(--elev-4)",
          display: "flex",
          flexDirection: "column",
        }}
      >
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: 9,
            padding: "12px 14px",
            borderBottom: "1px solid var(--border-default)",
          }}
        >
          <span aria-hidden="true" style={{ color: "var(--text-secondary)", display: "inline-flex" }}>
            <Inbox size={16} />
          </span>
          <span style={{ font: "var(--fw-semibold) var(--fs-sub) var(--font-sans)" }}>Task Inbox</span>
          <Badge mono style={{ color: "var(--text-muted)" }}>
            {tasksFixture.filter((t) => t.status !== "Done").length} open
          </Badge>
          <span style={{ marginLeft: "auto", display: "flex", gap: 4 }}>
            <IconButton label="Sync" size="sm" disabled>
              <RefreshCw size={15} />
            </IconButton>
            <IconButton label="Close" size="sm" onClick={onClose}>
              <X size={15} />
            </IconButton>
          </span>
        </div>
        <div style={{ padding: "8px 12px 0" }}>
          <Badge mono style={{ color: "var(--text-faint)" }}>
            display fixture — task intake lands with the Phase-7 connectors
          </Badge>
        </div>
        <div
          style={{
            display: "flex",
            gap: 4,
            padding: "8px 12px",
            borderBottom: "1px solid var(--border-subtle)",
            overflowX: "auto",
          }}
        >
          {TABS.map((t) => (
            <button
              key={t}
              type="button"
              aria-pressed={tab === t}
              onClick={() => setTab(t)}
              style={{
                whiteSpace: "nowrap",
                padding: "4px 9px",
                borderRadius: 999,
                cursor: "pointer",
                border: `1px solid ${tab === t ? "var(--accent-line)" : "var(--border-default)"}`,
                background: tab === t ? "var(--accent-surface)" : "transparent",
                color: tab === t ? "var(--accent-ink)" : "var(--text-muted)",
                font: "var(--fw-medium) var(--fs-meta) var(--font-sans)",
              }}
            >
              {t}
            </button>
          ))}
        </div>
        <div
          style={{ flex: 1, overflowY: "auto", padding: "8px 10px 14px", display: "flex", flexDirection: "column", gap: 7 }}
          data-testid="task-inbox-list"
        >
          {tasks.map((t) => {
            const s = SOURCE[t.source];
            return (
              <div
                key={t.source + t.id}
                title="Dispatch + drag-to-session arrive with the intent seam (daemon-gated)"
                style={{
                  border: "1px solid var(--border-default)",
                  borderRadius: "var(--r-2)",
                  background: "var(--surface-card)",
                  padding: "9px 10px",
                  display: "flex",
                  flexDirection: "column",
                  gap: 7,
                }}
              >
                <div style={{ display: "flex", alignItems: "center", gap: 7 }}>
                  <span
                    style={{
                      display: "inline-flex",
                      alignItems: "center",
                      gap: 4,
                      height: 16,
                      padding: "0 5px",
                      borderRadius: "var(--r-1)",
                      background: s.surf,
                      color: s.tone,
                      font: "var(--fw-medium) var(--fs-micro) var(--font-mono)",
                    }}
                  >
                    {s.icon} {t.id}
                  </span>
                  <span
                    style={{
                      marginLeft: "auto",
                      display: "inline-flex",
                      alignItems: "center",
                      gap: 4,
                      font: "var(--fs-micro) var(--font-mono)",
                      color: PRIO[t.priority],
                    }}
                  >
                    <span aria-hidden="true" style={{ width: 5, height: 5, borderRadius: 999, background: "currentColor" }} />
                    {t.priority}
                  </span>
                </div>
                <div style={{ font: "var(--fw-medium) var(--fs-label)/1.3 var(--font-sans)", color: "var(--text-primary)" }}>
                  {t.title}
                </div>
                <div style={{ display: "flex", alignItems: "center", gap: 5, flexWrap: "wrap" }}>
                  {t.labels.map((l) => (
                    <span
                      key={l}
                      style={{
                        font: "var(--fs-micro) var(--font-mono)",
                        color: "var(--text-muted)",
                        background: "var(--neutral-surface)",
                        padding: "1px 5px",
                        borderRadius: "var(--r-1)",
                      }}
                    >
                      {l}
                    </span>
                  ))}
                  {t.planTask ? (
                    <span style={{ font: "var(--fs-micro) var(--font-mono)", color: "var(--brain-ink)" }}>
                      ↳ {t.planTask}
                    </span>
                  ) : null}
                  {t.session ? (
                    <span style={{ marginLeft: "auto", font: "var(--fs-micro) var(--font-mono)", color: "var(--live-ink)" }}>
                      ● in session
                    </span>
                  ) : null}
                </div>
              </div>
            );
          })}
          {tasks.length === 0 ? (
            <div style={{ font: "var(--fs-meta) var(--font-sans)", color: "var(--text-faint)", padding: "12px 4px" }}>
              Nothing here.
            </div>
          ) : null}
        </div>
      </div>
    </Overlay>
  );
}

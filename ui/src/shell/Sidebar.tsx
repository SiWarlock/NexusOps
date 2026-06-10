import { useState, type CSSProperties, type ReactNode } from "react";
import {
  ChevronDown,
  ChevronRight,
  CircleAlert,
  CodeXml,
  FileDiff,
  Inbox,
  LayoutDashboard,
  LayoutGrid,
  ListChecks,
  Package,
  ScrollText,
  Terminal,
  UsersRound,
  Workflow,
} from "lucide-react";
import { describeStatus } from "../status/descriptors";
import { compareByAttention } from "../status/attention";
import { AttentionMarker } from "../status/AttentionMarker";
import { describeResumeMode } from "../recovery/model";
import type { ResumeMode } from "../contracts/index";
import type { ProjectActivityRow, SessionRow } from "../contracts/index";
import type { ProjectSwitcherCounts } from "./derive";
import type { ViewName } from "./view-history";
import {
  projectDisplayFixture,
  sessionDisplayFixture,
  sessionsByProject,
  type WorkflowTone,
} from "./display-meta";

const WF_TONE: Record<WorkflowTone, string> = {
  active: "var(--teal-ink)",
  drift: "var(--warning-ink)",
  "needs-personalization": "var(--caution-ink)",
  none: "var(--text-faint)",
};

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

/** The workspace view-nav entries (ported from kit-shell.jsx VIEWS). */
const VIEWS: { id: ViewName; label: string; icon: ReactNode }[] = [
  { id: "command", label: "Command Center", icon: <LayoutDashboard size={16} /> },
  { id: "graph", label: "Project Graph", icon: <Workflow size={16} /> },
  { id: "plan", label: "Plan", icon: <ListChecks size={16} /> },
  { id: "editor", label: "Editor", icon: <CodeXml size={16} /> },
  { id: "terminal", label: "Session Terminal", icon: <Terminal size={16} /> },
  { id: "code", label: "Code / Diff Review", icon: <FileDiff size={16} /> },
];
const PLATFORM_VIEWS: { id: ViewName; label: string; icon: ReactNode }[] = [
  { id: "packs", label: "Workflow Packs", icon: <Package size={16} /> },
  { id: "audit", label: "Audit Trail", icon: <ScrollText size={16} /> },
];

const navBtnStyle = (on: boolean): CSSProperties => ({
  display: "flex",
  alignItems: "center",
  gap: 8,
  width: "100%",
  height: 28,
  padding: "0 8px",
  borderRadius: "var(--r-2)",
  border: "none",
  cursor: "pointer",
  font: "var(--fw-medium) var(--fs-label) var(--font-sans)",
  textAlign: "left",
  background: on ? "var(--accent-surface)" : "transparent",
  color: on ? "var(--accent-ink)" : "var(--text-secondary)",
});

function NavBtn({
  id,
  label,
  icon,
  view,
  onNavigate,
}: {
  id: ViewName;
  label: string;
  icon: ReactNode;
  view: ViewName;
  onNavigate: (v: ViewName) => void;
}) {
  return (
    <button type="button" onClick={() => onNavigate(id)} style={navBtnStyle(view === id)} aria-current={view === id ? "page" : undefined}>
      <span aria-hidden="true" style={{ display: "inline-flex", flex: "none" }}>
        {icon}
      </span>
      {label}
    </button>
  );
}

// O-2 survival display on the dense sidebar: a compact resumed/replayed indicator
// (glyph + sr-only label — never color alone, §11.6). The full visible label lives
// in the Sessions table; here the distinct glyph SHAPE is the visible channel.
function SidebarResumeIndicator({ mode }: { mode: ResumeMode }) {
  const desc = describeResumeMode(mode);
  return (
    <span className="sidebar__resume-mode" data-resume-mode={mode} style={{ flex: "none", color: "var(--text-faint)", font: "10px var(--font-mono)" }}>
      <span aria-hidden="true">{desc.glyph}</span>
      <span className="sr-only">{desc.label}</span>
    </span>
  );
}

/** One project group in the workspace tree: header row + its session rows. */
function ProjectGroup({
  project,
  sessions,
  counts,
  defaultOpen,
  selectedSessionId,
  onOpenSession,
  resumeModes,
}: {
  project: ProjectActivityRow;
  sessions: SessionRow[];
  counts: ProjectSwitcherCounts;
  defaultOpen: boolean;
  selectedSessionId: string | null;
  onOpenSession: (s: SessionRow) => void;
  resumeModes: Record<string, ResumeMode>;
}) {
  const [exp, setExp] = useState(defaultOpen);
  const meta = projectDisplayFixture[project.project_id];
  // §11.3 sidebar weight: sessions within the group are attention-ordered.
  const ordered = sessions
    .map((s) => ({
      session: s,
      attentionRank: describeStatus("Session", s.status).attentionRank,
    }))
    .toSorted(compareByAttention)
    .map((r) => r.session);

  return (
    <div style={{ marginBottom: 2 }}>
      <button
        type="button"
        onClick={() => setExp(!exp)}
        aria-expanded={exp}
        style={{
          display: "flex",
          alignItems: "center",
          gap: 6,
          width: "100%",
          padding: "5px 6px",
          borderRadius: "var(--r-2)",
          border: "none",
          background: "transparent",
          cursor: "pointer",
          textAlign: "left",
        }}
      >
        <span aria-hidden="true" style={{ display: "inline-flex", color: "var(--text-faint)", flex: "none" }}>
          {exp ? <ChevronDown size={16} /> : <ChevronRight size={16} />}
        </span>
        <span
          style={{
            flex: 1,
            minWidth: 0,
            font: "var(--fw-medium) var(--fs-label) var(--font-sans)",
            color: "var(--text-primary)",
            overflow: "hidden",
            textOverflow: "ellipsis",
            whiteSpace: "nowrap",
          }}
        >
          {project.name}
        </span>
        {counts.waitingOnYou > 0 ? (
          <span
            title="waiting on human"
            style={{ width: 7, height: 7, borderRadius: 999, background: "var(--attention-solid)", flex: "none" }}
          />
        ) : null}
        {counts.activeSessions > 0 ? (
          <span style={{ font: "10px var(--font-mono)", color: "var(--text-muted)", flex: "none" }}>
            {counts.activeSessions}
          </span>
        ) : null}
        <span
          title={`workflow: ${meta?.workflow ?? "none"}`}
          style={{
            width: 6,
            height: 6,
            borderRadius: 2,
            background: WF_TONE[meta?.workflow ?? "none"],
            flex: "none",
          }}
        />
      </button>
      {exp
        ? ordered.map((s) => {
            const selected = selectedSessionId === s.session_id;
            const rank = describeStatus("Session", s.status).attentionRank;
            const display = sessionDisplayFixture[s.session_id];
            const resumeMode = resumeModes[s.session_id];
            return (
              <button
                key={s.session_id}
                type="button"
                data-item-id={`Session:${s.session_id}`}
                onClick={() => onOpenSession(s)}
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: 7,
                  width: "100%",
                  padding: "4px 6px 4px 22px",
                  borderRadius: "var(--r-2)",
                  border: "none",
                  cursor: "pointer",
                  textAlign: "left",
                  background: selected ? "var(--surface-active)" : "transparent",
                }}
              >
                <AttentionMarker rank={rank} variant="dot" />
                <span
                  style={{
                    flex: 1,
                    minWidth: 0,
                    font: "var(--fs-meta) var(--font-sans)",
                    color: selected ? "var(--text-primary)" : "var(--text-secondary)",
                    overflow: "hidden",
                    textOverflow: "ellipsis",
                    whiteSpace: "nowrap",
                  }}
                >
                  {s.title ?? s.session_id}
                </span>
                {display?.team ? (
                  <span
                    title="Agent team — open team view"
                    style={{
                      display: "inline-flex",
                      alignItems: "center",
                      gap: 3,
                      flex: "none",
                      height: 15,
                      padding: "0 5px",
                      borderRadius: "var(--r-1)",
                      background: "var(--teal-surface)",
                      border: "1px solid var(--teal-line)",
                      color: "var(--teal-ink)",
                      font: "var(--fw-semibold) 9px/1 var(--font-sans)",
                    }}
                  >
                    <UsersRound size={10} aria-hidden="true" /> Team
                  </span>
                ) : null}
                {resumeMode ? <SidebarResumeIndicator mode={resumeMode} /> : null}
              </button>
            );
          })
        : null}
    </div>
  );
}

/**
 * Left sidebar (ported from kit-shell.jsx Sidebar): the WORKSPACE project tree
 * (expandable per-project groups → attention-ordered session rows, §11.3), the
 * view nav (Command Center → Code/Diff Review), Human Input + Task Inbox, and
 * the PLATFORM nav (Workflow Packs, Audit Trail). Projects/sessions/counts are
 * projection-driven; repo/workflow/team decorations come from the display
 * side-maps (fixture until the daemon enriches the projections — flagged).
 */
export function Sidebar({
  projects = [],
  sessions = [],
  counts = {},
  view,
  onNavigate,
  selectedSessionId = null,
  onOpenSession = () => {},
  waiting = 0,
  resumeModes = {},
  onHumanInput,
  onTasks,
}: {
  projects?: ProjectActivityRow[];
  sessions?: SessionRow[];
  counts?: Record<string, ProjectSwitcherCounts>;
  view: ViewName;
  onNavigate: (v: ViewName) => void;
  selectedSessionId?: string | null;
  onOpenSession?: (s: SessionRow) => void;
  /** Global waiting-on-you count (Human Input badge). */
  waiting?: number;
  /** id-keyed side map (Lesson §8) — resume indicators without widening the row. */
  resumeModes?: Record<string, ResumeMode>;
  /** Opens the Human Input queue drawer (absent → disabled). */
  onHumanInput?: () => void;
  /** Opens the Task Inbox drawer (absent → disabled). */
  onTasks?: () => void;
}) {
  const [projectsExp, setProjectsExp] = useState(true);
  const grouped = sessionsByProject(sessions);
  const projectsOn = view === "projects";
  const ZERO: ProjectSwitcherCounts = { activeSessions: 0, openPRs: 0, waitingOnYou: 0 };

  return (
    <nav className="sidebar" aria-label="Sidebar">
      <div className="sidebar__scroll">
        <Eyebrow style={{ padding: "0 6px 6px" }}>Workspace</Eyebrow>

        {/* Projects: expandable tree (chevron) + click-into page (label). */}
        <div style={{ marginBottom: 2 }}>
          <div
            style={{
              display: "flex",
              alignItems: "center",
              height: 28,
              borderRadius: "var(--r-2)",
              background: projectsOn ? "var(--accent-surface)" : "transparent",
            }}
          >
            <button
              type="button"
              onClick={() => setProjectsExp((e) => !e)}
              aria-expanded={projectsExp}
              title={projectsExp ? "Collapse" : "Expand"}
              style={{
                display: "inline-flex",
                alignItems: "center",
                justifyContent: "center",
                width: 24,
                height: 28,
                border: "none",
                background: "transparent",
                cursor: "pointer",
                color: "var(--text-faint)",
                flex: "none",
              }}
            >
              <span aria-hidden="true" style={{ display: "inline-flex" }}>
                {projectsExp ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
              </span>
              <span className="sr-only">{projectsExp ? "Collapse projects" : "Expand projects"}</span>
            </button>
            <button
              type="button"
              onClick={() => onNavigate("projects")}
              style={{
                display: "flex",
                alignItems: "center",
                gap: 8,
                flex: 1,
                height: 28,
                padding: "0 8px 0 2px",
                border: "none",
                background: "transparent",
                cursor: "pointer",
                font: "var(--fw-medium) var(--fs-label) var(--font-sans)",
                textAlign: "left",
                color: projectsOn ? "var(--accent-ink)" : "var(--text-secondary)",
              }}
            >
              <span aria-hidden="true" style={{ display: "inline-flex", flex: "none" }}>
                <LayoutGrid size={16} />
              </span>
              Projects
              <span style={{ marginLeft: "auto", font: "10px var(--font-mono)", color: "var(--text-faint)" }}>
                {projects.length}
              </span>
            </button>
          </div>
          {projectsExp ? (
            <div style={{ margin: "2px 0 4px", paddingLeft: 4 }}>
              {projects.map((p, i) => (
                <ProjectGroup
                  key={p.project_id}
                  project={p}
                  sessions={grouped[p.project_id] ?? []}
                  counts={counts[p.project_id] ?? ZERO}
                  defaultOpen={i === 0}
                  selectedSessionId={selectedSessionId}
                  onOpenSession={onOpenSession}
                  resumeModes={resumeModes}
                />
              ))}
            </div>
          ) : null}
        </div>

        {VIEWS.map((v) => (
          <NavBtn key={v.id} {...v} view={view} onNavigate={onNavigate} />
        ))}

        <button
          type="button"
          disabled={!onHumanInput}
          onClick={onHumanInput}
          title="Human Input queue (⌘⇧H)"
          style={{
            display: "flex",
            alignItems: "center",
            gap: 8,
            width: "100%",
            height: 28,
            padding: "0 8px",
            borderRadius: "var(--r-2)",
            border: "none",
            cursor: onHumanInput ? "pointer" : "default",
            font: "var(--fw-medium) var(--fs-label) var(--font-sans)",
            background: "transparent",
            color: waiting > 0 ? "var(--attention-ink)" : "var(--text-faint)",
            textAlign: "left",
          }}
        >
          <span aria-hidden="true" style={{ display: "inline-flex", flex: "none" }}>
            <CircleAlert size={16} />
          </span>
          Human Input
          {waiting > 0 ? (
            <span
              data-testid="sidebar-waiting-badge"
              style={{
                marginLeft: "auto",
                padding: "0 6px",
                height: 16,
                display: "inline-flex",
                alignItems: "center",
                borderRadius: 999,
                background: "var(--attention-solid)",
                color: "var(--attention-on-solid)",
                font: "10px var(--font-mono)",
              }}
            >
              {waiting}
            </span>
          ) : null}
        </button>
        <button
          type="button"
          disabled={!onTasks}
          onClick={onTasks}
          title="Task Inbox (⌘⇧P)"
          style={{
            display: "flex",
            alignItems: "center",
            gap: 8,
            width: "100%",
            height: 28,
            padding: "0 8px",
            borderRadius: "var(--r-2)",
            border: "none",
            cursor: onTasks ? "pointer" : "default",
            font: "var(--fw-medium) var(--fs-label) var(--font-sans)",
            background: "transparent",
            color: "var(--text-secondary)",
            textAlign: "left",
          }}
        >
          <span aria-hidden="true" style={{ display: "inline-flex", flex: "none" }}>
            <Inbox size={16} />
          </span>
          Task Inbox
          <kbd
            style={{
              marginLeft: "auto",
              font: "9px var(--font-mono)",
              color: "var(--text-faint)",
              border: "1px solid var(--border-default)",
              borderRadius: 3,
              padding: "0 4px",
            }}
          >
            ⌘⇧P
          </kbd>
        </button>

        <Eyebrow style={{ padding: "12px 6px 6px" }}>Platform</Eyebrow>
        {PLATFORM_VIEWS.map((v) => (
          <NavBtn key={v.id} {...v} view={view} onNavigate={onNavigate} />
        ))}
      </div>
    </nav>
  );
}

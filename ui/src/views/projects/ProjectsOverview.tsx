import { ArrowRight, FolderGit2, GitPullRequest, LayoutGrid, Package, Plus } from "lucide-react";
import type { ProjectActivityRow } from "../../contracts/index";
import { Badge, Button } from "../../design-system/kit";
import type { ProjectSwitcherCounts } from "../../shell/derive";
import { projectDisplayFixture, type WorkflowTone } from "../../shell/display-meta";

const ZERO: ProjectSwitcherCounts = { activeSessions: 0, openPRs: 0, waitingOnYou: 0 };

// workflow tone → [label, ink, surface, line] (the prototype's WF_TONE).
const WF_TONE: Record<WorkflowTone, [string, string, string, string]> = {
  active: ["Active", "var(--teal-ink)", "var(--teal-surface)", "var(--teal-line)"],
  drift: ["Drift", "var(--warning-ink)", "var(--warning-surface)", "var(--warning-line)"],
  "needs-personalization": [
    "Needs setup",
    "var(--caution-ink)",
    "var(--caution-surface)",
    "var(--caution-line)",
  ],
  none: ["No pack", "var(--text-faint)", "var(--neutral-surface)", "var(--border-default)"],
};

/** Honest, non-optimistic feedback for the add-project flow (the daemon ack / verbatim §6.4
 *  rejection — never a synthesized success). Rendered as a `role="status"` line, glyph+label. */
export type AddProjectNotice = { kind: "ok" | "error"; message: string };

/**
 * Projects overview (ported from kit-views.jsx ProjectsOverview): the top-layer
 * project card grid — name + repo, live counters (active sessions · open PRs ·
 * waiting badge, derived from REAL projections), and the workflow-pack chip
 * (display side-map — fixture until the WorkflowInstance projection lands).
 * Card click selects the project and returns to its Command Center. "Add project"
 * submits a `project.rescan` intent (wired by `ProjectsOverviewContainer`) — gated
 * on `canAddProject` (the fail-safe READ-ONLY gate, forbidden #6); disabled-not-faked
 * when the daemon is unavailable.
 */
export function ProjectsOverview({
  projects,
  counts,
  activeProjectId,
  onSelectProject,
  canAddProject = false,
  onAddProject,
  addProjectNotice = null,
}: {
  projects: ProjectActivityRow[];
  counts: Record<string, ProjectSwitcherCounts>;
  activeProjectId: string | null;
  onSelectProject: (id: string) => void;
  /** Fail-safe gate: the button is disabled unless the daemon is live + not mid-submit. */
  canAddProject?: boolean;
  onAddProject?: () => void;
  addProjectNotice?: AddProjectNotice | null;
}) {
  return (
    <div
      aria-label="Projects Overview"
      style={{ height: "100%", overflowY: "auto", background: "var(--surface-canvas)" }}
    >
      <div
        style={{
          padding: 16,
          borderBottom: "1px solid var(--border-subtle)",
          display: "flex",
          alignItems: "center",
          gap: 10,
        }}
      >
        <span aria-hidden="true" style={{ color: "var(--text-secondary)", display: "inline-flex" }}>
          <LayoutGrid size={18} />
        </span>
        <h1 style={{ margin: 0, font: "var(--fw-semibold) var(--fs-h2)/1 var(--font-sans)", letterSpacing: "var(--tracking-tight)" }}>
          Projects
        </h1>
        <Badge tone="neutral" mono>
          {projects.length}
        </Badge>
        <div style={{ marginLeft: "auto", display: "flex", alignItems: "center", gap: 10 }}>
          {addProjectNotice ? (
            <span
              role="status"
              style={{
                font: "var(--fs-meta) var(--font-sans)",
                color: addProjectNotice.kind === "error" ? "var(--warning-ink)" : "var(--text-muted)",
                display: "inline-flex",
                alignItems: "center",
                gap: 5,
                maxWidth: 320,
              }}
            >
              {/* glyph + label — never color alone (§11) */}
              <span aria-hidden="true">{addProjectNotice.kind === "error" ? "⚠" : "✓"}</span>
              {addProjectNotice.message}
            </span>
          ) : null}
          <span
            title={
              canAddProject
                ? "Add a git repository as a project"
                : "Connect to the daemon to add a project"
            }
          >
            <Button
              variant="primary"
              size="sm"
              icon={<Plus size={14} />}
              disabled={!canAddProject}
              onClick={onAddProject}
            >
              Add project
            </Button>
          </span>
        </div>
      </div>
      <div
        style={{
          padding: 16,
          display: "grid",
          gridTemplateColumns: "repeat(auto-fill, minmax(300px, 1fr))",
          gap: 12,
        }}
      >
        {projects.map((p) => {
          const c = counts[p.project_id] ?? ZERO;
          const meta = projectDisplayFixture[p.project_id];
          const wf = WF_TONE[meta?.workflow ?? "none"];
          const active = activeProjectId === p.project_id;
          return (
            <button
              key={p.project_id}
              type="button"
              data-item-id={`project:${p.project_id}`}
              onClick={() => onSelectProject(p.project_id)}
              style={{
                textAlign: "left",
                cursor: "pointer",
                border: `1px solid ${active ? "var(--accent-line)" : "var(--border-default)"}`,
                borderRadius: "var(--r-3)",
                background: "var(--surface-card)",
                padding: "14px 15px",
                display: "flex",
                flexDirection: "column",
                gap: 11,
                boxShadow: active ? "inset 0 0 0 1px var(--accent-line)" : "none",
              }}
            >
              <span style={{ display: "flex", alignItems: "flex-start", gap: 9 }}>
                <span
                  aria-hidden="true"
                  style={{
                    width: 30,
                    height: 30,
                    flex: "none",
                    borderRadius: "var(--r-2)",
                    background: "var(--surface-active)",
                    color: "var(--text-secondary)",
                    display: "inline-flex",
                    alignItems: "center",
                    justifyContent: "center",
                  }}
                >
                  <FolderGit2 size={16} />
                </span>
                <span style={{ minWidth: 0, flex: 1 }}>
                  <span
                    style={{
                      display: "block",
                      font: "var(--fw-semibold) var(--fs-body-lg)/1.25 var(--font-sans)",
                      color: "var(--text-primary)",
                    }}
                  >
                    {p.name}
                  </span>
                  <span
                    style={{
                      display: "block",
                      font: "var(--fs-meta) var(--font-mono)",
                      color: "var(--text-faint)",
                      marginTop: 2,
                    }}
                  >
                    {meta?.repo ?? p.project_id}
                  </span>
                </span>
                {c.waitingOnYou > 0 ? (
                  <span
                    title="waiting on human"
                    style={{
                      display: "inline-flex",
                      alignItems: "center",
                      gap: 4,
                      height: 18,
                      padding: "0 6px",
                      borderRadius: 999,
                      background: "var(--attention-solid)",
                      color: "var(--attention-on-solid)",
                      font: "var(--fw-semibold) var(--fs-micro) var(--font-mono)",
                    }}
                  >
                    ◆ {c.waitingOnYou}
                  </span>
                ) : null}
              </span>
              <span
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: 14,
                  font: "var(--fs-meta) var(--font-mono)",
                  color: "var(--text-muted)",
                }}
              >
                <span style={{ display: "inline-flex", alignItems: "center", gap: 5 }}>
                  <span
                    aria-hidden="true"
                    style={{ width: 6, height: 6, borderRadius: 999, background: "var(--live-solid)" }}
                  />
                  {c.activeSessions} active
                </span>
                <span style={{ display: "inline-flex", alignItems: "center", gap: 5 }}>
                  <GitPullRequest size={12} aria-hidden="true" />
                  {c.openPRs} PR
                </span>
              </span>
              <span style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <span
                  style={{
                    display: "inline-flex",
                    alignItems: "center",
                    gap: 5,
                    height: 20,
                    padding: "0 8px",
                    borderRadius: "var(--r-1)",
                    background: wf[2],
                    border: `1px solid ${wf[3]}`,
                    color: wf[1],
                    font: "var(--fw-medium) var(--fs-micro) var(--font-sans)",
                  }}
                >
                  <Package size={12} aria-hidden="true" /> {wf[0]}
                </span>
                <span
                  style={{
                    marginLeft: "auto",
                    font: "var(--fs-meta) var(--font-sans)",
                    color: "var(--accent-ink)",
                    display: "inline-flex",
                    alignItems: "center",
                    gap: 3,
                  }}
                >
                  Open <ArrowRight size={13} aria-hidden="true" />
                </span>
              </span>
            </button>
          );
        })}
      </div>
    </div>
  );
}

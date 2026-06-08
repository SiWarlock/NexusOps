import { useState } from "react";
import type { ProjectActivityRow } from "../contracts/index";
import type { ProjectSwitcherCounts } from "./derive";
import { useActiveProject } from "./active-project";

const ZERO: ProjectSwitcherCounts = {
  activeSessions: 0,
  openPRs: 0,
  waitingOnYou: 0,
};

/**
 * Project switcher (§11.2): a dropdown-popover over the ProjectActivity projection.
 * A trigger button (active project name + caret, `aria-haspopup="listbox"`) opens a
 * WAI-ARIA `listbox` of the projects with their live counts (active sessions · open
 * PRs · waiting-on-you). SINGLE-SELECT — choosing an option sets it active (UI
 * selection state via ActiveProjectContext; a read/scope selection, NOT a daemon
 * mutation — Lesson §13). The active option carries `aria-selected` + a ✓ glyph+label
 * (never color alone, §11.6). At zero projects the trigger is disabled ("No project")
 * — wire-or-disable, never a dead click. Counts are derived, never invented.
 *
 * L1 (this layer): the popover shell + click-to-select. Roving tabindex + keyboard
 * (Arrow/Home/End/Enter/Escape) + open-focus + click-outside + focus-return are L2.
 */
export function ProjectSwitcher({
  projects,
  counts,
}: {
  projects: ProjectActivityRow[];
  counts: Record<string, ProjectSwitcherCounts>;
}) {
  const { activeProjectId, setActiveProject } = useActiveProject();
  const [open, setOpen] = useState(false);
  const hasProjects = projects.length > 0;
  const activeProject = projects.find((p) => p.project_id === activeProjectId);
  const triggerLabel = hasProjects
    ? (activeProject?.name ?? "Select project")
    : "No project";

  const select = (id: string) => {
    setActiveProject(id);
    setOpen(false);
  };

  return (
    <div className="project-switcher">
      <button
        type="button"
        className="project-switcher__trigger"
        aria-haspopup="listbox"
        aria-expanded={open}
        disabled={!hasProjects}
        onClick={() => setOpen((o) => !o)}
      >
        <span className="project-switcher__trigger-name">{triggerLabel}</span>
        <span aria-hidden="true" className="project-switcher__caret">
          ▾
        </span>
      </button>
      {open && hasProjects ? (
        <ul role="listbox" aria-label="Projects" className="project-switcher__listbox">
          {projects.map((project) => {
            const c = counts[project.project_id] ?? ZERO;
            const isActive = project.project_id === activeProjectId;
            return (
              <li
                key={project.project_id}
                role="option"
                className="project-switcher__item"
                data-project-id={project.project_id}
                aria-selected={isActive}
                onClick={() => select(project.project_id)}
                // Self-contained accessible name (project + counts + active state).
                // The visible count glyphs below are aria-hidden (no double-read).
                // Worded to avoid "sessions"/"pull requests" so the name doesn't
                // collide with the view-switch buttons.
                aria-label={`${project.name}${isActive ? " (active project)" : ""} — ${c.activeSessions} active, ${c.openPRs} open PRs, ${c.waitingOnYou} waiting`}
              >
                <span className="project-switcher__name">{project.name}</span>
                {isActive ? (
                  <span className="project-switcher__active">✓ Active</span>
                ) : null}
                <span className="project-switcher__counts" aria-hidden="true">
                  <span title="active sessions">▶ {c.activeSessions}</span>{" "}
                  <span title="open PRs">⇡ {c.openPRs}</span>{" "}
                  <span title="waiting on you">● {c.waitingOnYou}</span>
                </span>
              </li>
            );
          })}
        </ul>
      ) : null}
    </div>
  );
}

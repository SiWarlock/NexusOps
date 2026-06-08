import type { ProjectActivityRow } from "../contracts/index";
import type { ProjectSwitcherCounts } from "./derive";
import { useActiveProject } from "./active-project";

const ZERO: ProjectSwitcherCounts = {
  activeSessions: 0,
  openPRs: 0,
  waitingOnYou: 0,
};

/**
 * Project switcher: lists exactly the projects from the ProjectActivity
 * projection with their live counts (active sessions · open PRs · waiting-on-you).
 * SINGLE-SELECT — clicking a project sets it active (UI selection state via the
 * ActiveProjectContext; a read/scope selection, NOT a daemon mutation). The active
 * project carries `aria-pressed` + a glyph+label "✓ Active" indicator (never color
 * alone, §11.6). The full dropdown-popover widget is deferred (a presentation
 * polish) — this is the functional selector. Counts are derived, never invented.
 */
export function ProjectSwitcher({
  projects,
  counts,
}: {
  projects: ProjectActivityRow[];
  counts: Record<string, ProjectSwitcherCounts>;
}) {
  const { activeProjectId, setActiveProject } = useActiveProject();
  return (
    <div className="project-switcher" role="group" aria-label="Project switcher">
      {projects.map((project) => {
        const c = counts[project.project_id] ?? ZERO;
        const isActive = project.project_id === activeProjectId;
        return (
          <button
            key={project.project_id}
            type="button"
            className="project-switcher__item"
            data-project-id={project.project_id}
            aria-pressed={isActive}
            onClick={() => setActiveProject(project.project_id)}
            // Self-contained accessible name (project + counts + active state). The
            // counts are spelled out here so AT gets them; the visible glyphs below
            // are aria-hidden (no double-read). Worded to avoid "sessions"/"pull
            // requests" so the name doesn't collide with the view-switch buttons.
            aria-label={`${project.name}${isActive ? " (active project)" : ""} — ${c.activeSessions} active, ${c.openPRs} open PRs, ${c.waitingOnYou} waiting`}
          >
            <span className="project-switcher__name">{project.name}</span>
            {isActive ? (
              <span className="project-switcher__active">✓ Active</span>
            ) : null}
            {/* Counts: glyph + number (never color-alone, §11.1); aria-hidden — the
                button's aria-label carries them for AT. */}
            <span className="project-switcher__counts" aria-hidden="true">
              <span title="active sessions">▶ {c.activeSessions}</span>{" "}
              <span title="open PRs">⇡ {c.openPRs}</span>{" "}
              <span title="waiting on you">● {c.waitingOnYou}</span>
            </span>
          </button>
        );
      })}
    </div>
  );
}

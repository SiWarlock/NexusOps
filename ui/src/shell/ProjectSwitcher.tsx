import type { ProjectActivityRow } from "../contracts/index";
import type { ProjectSwitcherCounts } from "./derive";

const ZERO: ProjectSwitcherCounts = {
  activeSessions: 0,
  openPRs: 0,
  waitingOnYou: 0,
};

/**
 * Project switcher: lists exactly the projects from the ProjectActivity
 * projection with their live counts (active sessions · open PRs · waiting-on-you).
 * Counts are derived (deriveProjectSwitcherCounts), never invented.
 */
export function ProjectSwitcher({
  projects,
  counts,
}: {
  projects: ProjectActivityRow[];
  counts: Record<string, ProjectSwitcherCounts>;
}) {
  return (
    <div className="project-switcher" role="group" aria-label="Project switcher">
      {projects.map((project) => {
        const c = counts[project.project_id] ?? ZERO;
        return (
          <div
            key={project.project_id}
            className="project-switcher__item"
            data-project-id={project.project_id}
          >
            <span className="project-switcher__name">{project.name}</span>
            {/* Counts carry both glyph + number (never color-alone, §11.1). */}
            <span
              className="project-switcher__counts"
              aria-label={`${c.activeSessions} active sessions, ${c.openPRs} open pull requests, ${c.waitingOnYou} waiting on you`}
            >
              <span title="active sessions">▶ {c.activeSessions}</span>{" "}
              <span title="open PRs">⇡ {c.openPRs}</span>{" "}
              <span title="waiting on you">● {c.waitingOnYou}</span>
            </span>
          </div>
        );
      })}
    </div>
  );
}

import {
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type KeyboardEvent,
} from "react";
import type { ProjectActivityRow } from "../contracts/index";
import type { ProjectSwitcherCounts } from "./derive";
import { useActiveProject } from "./active-project";
import { isRovingKey, nextTabIndex } from "../a11y/roving";

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
 * Keyboard (APG Listbox, manual activation): the listbox uses roving tabindex
 * (`nextTabIndex`, vertical) — exactly one option is the tabstop. Open focuses the
 * active option (first if none); Arrow/Home/End move focus; Enter/Space select +
 * close; Escape closes without selecting; both return focus to the trigger; an
 * outside click closes without selecting.
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
  const rootRef = useRef<HTMLDivElement | null>(null);
  const triggerRef = useRef<HTMLButtonElement | null>(null);
  const optionRefs = useRef<(HTMLLIElement | null)[]>([]);

  const hasProjects = projects.length > 0;
  const activeProject = projects.find((p) => p.project_id === activeProjectId);
  const triggerLabel = hasProjects
    ? (activeProject?.name ?? "Select project")
    : "No project";
  // The roving cursor starts at the active option (first if none active).
  const activeIndex = Math.max(
    0,
    projects.findIndex((p) => p.project_id === activeProjectId),
  );
  const [focusIndex, setFocusIndex] = useState(activeIndex);

  const openMenu = () => {
    setFocusIndex(activeIndex);
    setOpen(true);
  };
  const closeMenu = (returnFocus = true) => {
    setOpen(false);
    if (returnFocus) triggerRef.current?.focus();
  };
  const select = (id: string) => {
    setActiveProject(id);
    closeMenu(true);
  };

  // Open → focus the active option (APG). useLayoutEffect so focus lands before
  // paint (no flash). Fires ONLY on the open transition (deps [open]) — NOT on
  // later activeIndex changes, so a background projection update can't yank focus
  // from a user navigating the open list. `activeIndex` is read fresh at open.
  useLayoutEffect(() => {
    if (open) optionRefs.current[activeIndex]?.focus();
    // eslint-disable-next-line react-hooks/exhaustive-deps -- focus once on open; arrows manage focus thereafter
  }, [open]);

  // Click-outside closes the popover (no selection change, no forced focus return).
  useEffect(() => {
    if (!open) return;
    const onDocMouseDown = (e: MouseEvent) => {
      if (rootRef.current && !rootRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    document.addEventListener("mousedown", onDocMouseDown);
    return () => document.removeEventListener("mousedown", onDocMouseDown);
  }, [open]);

  const onListboxKeyDown = (e: KeyboardEvent<HTMLUListElement>) => {
    if (e.key === "Escape") {
      e.preventDefault();
      closeMenu(true);
      return;
    }
    // The focused option is the authoritative current index (read from the DOM at
    // event time — never a stale closure, Lesson §9); fall back to the roving
    // cursor if focus somehow isn't on an option.
    const focused = optionRefs.current.findIndex((el) => el === document.activeElement);
    const current = focused >= 0 ? focused : focusIndex;
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      const project = projects[current]; // guarded — the list can shrink while open
      if (project) select(project.project_id);
      return;
    }
    if (isRovingKey(e.key, "vertical")) {
      e.preventDefault();
      const next = nextTabIndex(current, projects.length, e.key, "vertical");
      setFocusIndex(next);
      optionRefs.current[next]?.focus();
    }
  };

  return (
    <div className="project-switcher" ref={rootRef}>
      <button
        ref={triggerRef}
        type="button"
        className="project-switcher__trigger"
        aria-haspopup="listbox"
        aria-expanded={open}
        disabled={!hasProjects}
        onClick={() => (open ? closeMenu(true) : openMenu())}
      >
        <span className="project-switcher__trigger-name">{triggerLabel}</span>
        <span aria-hidden="true" className="project-switcher__caret">
          ▾
        </span>
      </button>
      {open && hasProjects ? (
        <ul
          role="listbox"
          aria-label="Projects"
          className="project-switcher__listbox"
          onKeyDown={onListboxKeyDown}
        >
          {projects.map((project, i) => {
            const c = counts[project.project_id] ?? ZERO;
            const isActive = project.project_id === activeProjectId;
            return (
              <li
                key={project.project_id}
                ref={(el) => {
                  optionRefs.current[i] = el;
                }}
                role="option"
                className="project-switcher__item"
                data-project-id={project.project_id}
                aria-selected={isActive}
                tabIndex={i === focusIndex ? 0 : -1}
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

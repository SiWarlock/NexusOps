import {
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type KeyboardEvent,
} from "react";
import { Check, ChevronsUpDown, FolderGit2 } from "lucide-react";
import type { ProjectActivityRow } from "../contracts/index";
import type { ProjectSwitcherCounts } from "./derive";
import { projectDisplayFixture, type WorkflowTone } from "./display-meta";
import { projectLabel } from "./project-label";
import { useActiveProject } from "./active-project";
import { isRovingKey, nextTabIndex } from "../a11y/roving";

const ZERO: ProjectSwitcherCounts = {
  activeSessions: 0,
  openPRs: 0,
  waitingOnYou: 0,
};

// Workflow-instance tone → dot color (kit semantic tokens; the prototype's
// wfTone map). The dot is title-labelled — color is additive, not the channel.
const WF_TONE: Record<WorkflowTone, string> = {
  active: "var(--teal-ink)",
  drift: "var(--warning-ink)",
  "needs-personalization": "var(--caution-ink)",
  none: "var(--text-faint)",
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
 *
 * Visual anatomy ported from kit-shell.jsx ProjectSwitcher. (The prototype's
 * "All projects" scope is a deliberate deviation-out: the active-project model is
 * single-project — an "all" scope is a model change, not styling. Flagged.)
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
  const triggerLabel = !hasProjects
    ? "No project"
    : activeProject
      ? projectLabel(activeProject) // name, or an id-fallback for a name-less daemon row
      : "Select project";
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
    <div className="project-switcher" ref={rootRef} style={{ position: "relative" }}>
      <button
        ref={triggerRef}
        type="button"
        className="project-switcher__trigger"
        aria-haspopup="listbox"
        aria-expanded={open}
        disabled={!hasProjects}
        title="Switch project"
        onClick={() => (open ? closeMenu(true) : openMenu())}
        style={{
          display: "inline-flex",
          alignItems: "center",
          gap: 7,
          height: 28,
          padding: "0 9px",
          borderRadius: "var(--r-2)",
          border: `1px solid ${open ? "var(--border-strong)" : "var(--border-default)"}`,
          background: "var(--surface-input)",
          cursor: hasProjects ? "pointer" : "default",
        }}
      >
        <FolderGit2 size={14} aria-hidden="true" style={{ color: "var(--text-muted)", flex: "none" }} />
        <span
          className="project-switcher__trigger-name"
          style={{
            font: "var(--fw-medium) var(--fs-label)/1 var(--font-sans)",
            color: "var(--text-primary)",
            maxWidth: 220,
            overflow: "hidden",
            textOverflow: "ellipsis",
            whiteSpace: "nowrap",
          }}
        >
          {triggerLabel}
        </span>
        <span aria-hidden="true" className="project-switcher__caret" style={{ display: "inline-flex" }}>
          <ChevronsUpDown size={13} style={{ color: "var(--text-faint)" }} />
        </span>
      </button>
      {open && hasProjects ? (
        <ul
          role="listbox"
          aria-label="Projects"
          className="project-switcher__listbox"
          onKeyDown={onListboxKeyDown}
          style={{
            position: "absolute",
            top: 32,
            left: 0,
            zIndex: "var(--z-popover)" as unknown as number,
            width: 320,
            margin: 0,
            listStyle: "none",
            background: "var(--surface-card)",
            border: "1px solid var(--border-strong)",
            borderRadius: "var(--r-3)",
            boxShadow: "var(--elev-3)",
            overflow: "hidden",
            padding: 5,
            animation: "cp-pop-in 0.14s var(--ease-out)",
          }}
        >
          <li
            aria-hidden="true"
            style={{
              font: "var(--fw-semibold) var(--fs-micro) var(--font-sans)",
              letterSpacing: "var(--tracking-caps)",
              textTransform: "uppercase",
              color: "var(--text-faint)",
              padding: "6px 8px 4px",
            }}
          >
            Switch project
          </li>
          {projects.map((project, i) => {
            const c = counts[project.project_id] ?? ZERO;
            const meta = projectDisplayFixture[project.project_id];
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
                aria-label={`${projectLabel(project)}${isActive ? " (active project)" : ""} — ${c.activeSessions} active, ${c.openPRs} open PRs, ${c.waitingOnYou} waiting`}
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: 9,
                  width: "100%",
                  padding: "7px 8px",
                  borderRadius: "var(--r-2)",
                  border: "none",
                  cursor: "pointer",
                  textAlign: "left",
                  background: isActive ? "var(--accent-surface)" : "transparent",
                }}
              >
                <FolderGit2
                  size={15}
                  aria-hidden="true"
                  style={{ color: isActive ? "var(--accent-ink)" : "var(--text-muted)", flex: "none" }}
                />
                <span style={{ flex: 1, minWidth: 0 }}>
                  <span
                    className="project-switcher__name"
                    style={{
                      display: "block",
                      font: "var(--fw-medium) var(--fs-label) var(--font-sans)",
                      color: isActive ? "var(--accent-ink)" : "var(--text-primary)",
                      overflow: "hidden",
                      textOverflow: "ellipsis",
                      whiteSpace: "nowrap",
                    }}
                  >
                    {projectLabel(project)}
                  </span>
                  <span
                    style={{
                      display: "block",
                      font: "var(--fs-micro) var(--font-mono)",
                      color: "var(--text-faint)",
                    }}
                  >
                    {meta?.repo ?? project.project_id}
                  </span>
                </span>
                <span className="project-switcher__counts" aria-hidden="true" style={{ display: "inline-flex", alignItems: "center", gap: 7 }}>
                  {c.waitingOnYou > 0 ? (
                    <span
                      title="waiting"
                      style={{ width: 7, height: 7, borderRadius: 999, background: "var(--attention-solid)" }}
                    />
                  ) : null}
                  {c.activeSessions > 0 ? (
                    <span style={{ font: "10px var(--font-mono)", color: "var(--text-muted)" }}>
                      {c.activeSessions}
                    </span>
                  ) : null}
                  <span
                    title={`workflow: ${meta?.workflow ?? "none"}`}
                    style={{
                      width: 6,
                      height: 6,
                      borderRadius: 2,
                      background: WF_TONE[meta?.workflow ?? "none"],
                    }}
                  />
                </span>
                {isActive ? (
                  <span className="project-switcher__active" style={{ display: "inline-flex", color: "var(--accent-ink)" }}>
                    <Check size={14} aria-hidden="true" />
                    <span className="sr-only">Active</span>
                  </span>
                ) : null}
              </li>
            );
          })}
        </ul>
      ) : null}
    </div>
  );
}

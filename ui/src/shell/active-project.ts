// The UI active-project selection model (§11.2 TopBar switcher, §4.2 views filter
// by UI selection). PURE UI selection state over the FROZEN projects projection —
// NOT a daemon mutation, NOT a provisional contract, NOT a Gateway intent (a
// read/scope selection, so no canSubmitIntent gate). Mirrors the ReadOnlyProvider
// pattern (createElement, not JSX, so the pure predicates import freely).
import { createContext, createElement, useContext, type ReactNode } from "react";
import type { ProjectActivityRow } from "../contracts/index";

/** The default active scope: the first project when any exist, else null (zero-projects). */
export function defaultActiveProject(projects: ProjectActivityRow[]): string | null {
  return projects[0]?.project_id ?? null;
}

/**
 * Resolve the effective active project from a raw selection + the current
 * projects: the raw pick when it still exists in the projection, else the default
 * (first). Guards the stale-ID case — a selected project removed from the
 * projection re-scopes to the default instead of rooting views at a ghost id.
 */
export function resolveActiveProject(
  projects: ProjectActivityRow[],
  rawActiveProjectId: string | null,
): string | null {
  if (
    rawActiveProjectId !== null &&
    projects.some((p) => p.project_id === rawActiveProjectId)
  ) {
    return rawActiveProjectId;
  }
  return defaultActiveProject(projects);
}

/**
 * Scope rows to the active project; null active = unscoped (all rows — the
 * no-active treatment). Under a non-null active, a row with no `project_id`
 * (unassigned — `undefined` OR an explicit `null` from a projection-row shadow)
 * is EXCLUDED — it belongs to no project, so it isn't in the active project's scope.
 */
export function filterByActiveProject<T extends { project_id?: string | null }>(
  rows: T[],
  activeProjectId: string | null,
): T[] {
  if (activeProjectId === null) return rows;
  return rows.filter((row) => row.project_id === activeProjectId);
}

export interface ActiveProjectValue {
  activeProjectId: string | null;
  setActiveProject: (id: string) => void;
}

const ActiveProjectContext = createContext<ActiveProjectValue>({
  activeProjectId: null,
  setActiveProject: () => {},
});

export function ActiveProjectProvider(props: {
  value: ActiveProjectValue;
  children: ReactNode;
}) {
  return createElement(
    ActiveProjectContext.Provider,
    { value: props.value },
    props.children,
  );
}

export function useActiveProject(): ActiveProjectValue {
  return useContext(ActiveProjectContext);
}

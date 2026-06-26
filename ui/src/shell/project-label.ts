// The single source of a project's display label across the switcher / header / overview card —
// so a name-less daemon row (proj_project_activity is a COUNTERS table with NO `name` col, §7.2)
// renders ONE consistent, honest, interim id-derived label everywhere (no divergent fallbacks).

/** A project row carries an id; `name` is optional (the daemon serves a name-less counters table). */
type ProjectLabelRow = { project_id: string; name?: string | null };

/**
 * The interim label when a project has no name: honest, unique, clearly NOT a friendly name.
 * The last 6 chars of the id (the ULID tail) keep it short + collision-free. The daemon friendly
 * name (slice 092 serves the basename as `name`) supersedes this via `projectLabel` with ZERO rework.
 */
export function projectIdFallback(projectId: string): string {
  return `Project ${projectId.slice(-6)}`;
}

/** A project's display label: the real `name` when present (non-blank), else the id-fallback. */
export function projectLabel(row: ProjectLabelRow): string {
  const name = row.name?.trim();
  return name ? name : projectIdFallback(row.project_id);
}

/**
 * The active-project HEADER label: `"No project"` ONLY when there is no active project; an active
 * project with no name renders the id-fallback — never conflated with no-project (the bug being
 * fixed: the old `name ?? "No project"` showed "No project" for an active-but-nameless project).
 */
export function headerProjectLabel(active: ProjectLabelRow | null | undefined): string {
  return active ? projectLabel(active) : "No project";
}

// The Sessions table model (§11.2 dense list + §5.2 attention ordering): a pure
// derivation of the Session projection into sortable rows. Each session → a row
// enriched with its attention rank (descriptor table — single source) and its
// project name (a pure project_id→ProjectActivity.name join, §4.2 — never invented
// state). Base shape via the L1 toSessionItems mapper (no inline re-map — §8).
import type {
  ProjectActivityRow,
  ResumeMode,
  SessionRow,
} from "../../contracts/index";
import { compareByAttention, type AttentionRank } from "../../status/attention";
import { describeStatus } from "../../status/descriptors";
import { toSessionItems } from "../../projections/items";

export type SessionsSortKey = "attention" | "name" | "status" | "project";
export type SortDir = "asc" | "desc";

export interface SessionRowVM {
  id: string;
  machine: string;
  status: string;
  label: string;
  attentionRank: AttentionRank;
  projectName: string;
  /** O-2 survival: how the session came back after a restart (undefined if N/A). */
  resumeMode?: ResumeMode;
}

export interface SessionsSort {
  key: SessionsSortKey;
  dir: SortDir;
}

/** Attention-first by default (§5.2): needs-attention sessions at the top. */
export const DEFAULT_SORT: SessionsSort = { key: "attention", dir: "desc" };

/** A column's natural first-click direction: attention defaults desc, text asc. */
export function naturalDir(key: SessionsSortKey): SortDir {
  return key === "attention" ? "desc" : "asc";
}

/** Absent project_id → a visible marker (never a blank/crash). */
const NO_PROJECT = "—";

export function buildSessionRows(
  sessions: SessionRow[],
  projects: ProjectActivityRow[],
): SessionRowVM[] {
  const nameByProjectId = new Map(projects.map((p) => [p.project_id, p.name]));
  // toSessionItems is a 1:1 order-preserving map (§8), so the i-th item is the
  // i-th session — pair positionally to enrich without re-mapping the row inline.
  return toSessionItems(sessions).map((item, i) => {
    const session = sessions[i];
    const projectId = session?.project_id;
    return {
      ...item,
      attentionRank: describeStatus(item.machine, item.status).attentionRank,
      projectName:
        projectId == null
          ? NO_PROJECT
          : // unmatched project → the raw id stays visible
            (nameByProjectId.get(projectId) ?? projectId),
      // coerce the frozen SessionRow's explicit `null` (absent resume_mode) to undefined — the VM's
      // optional resumeMode is "present-or-absent", and the indicator renders only when present (ui-062).
      resumeMode: session?.resume_mode ?? undefined,
    };
  });
}

// The comparable text for a text sort key (attention is ranked separately).
function textField(
  row: SessionRowVM,
  key: Exclude<SessionsSortKey, "attention">,
): string {
  switch (key) {
    case "name":
      return row.label;
    case "status":
      return row.status;
    case "project":
      return row.projectName;
  }
}

// Directional comparator for one key. Attention reuses compareByAttention (§5
// single source); text keys use a FIXED "en" collation (deterministic across OS
// locales). `dir` flips the result.
function compareByKey(
  a: SessionRowVM,
  b: SessionRowVM,
  key: SessionsSortKey,
  dir: SortDir,
): number {
  if (key === "attention") {
    const c = compareByAttention(a, b); // desc = needs-attention first
    return dir === "desc" ? c : -c;
  }
  const c = textField(a, key).localeCompare(textField(b, key), "en");
  return dir === "asc" ? c : -c;
}

export function sortSessionRows(
  rows: SessionRowVM[],
  key: SessionsSortKey,
  dir: SortDir,
): SessionRowVM[] {
  return rows.toSorted((a, b) => {
    const primary = compareByKey(a, b, key, dir);
    // stable secondary tiebreak (id), DIRECTION-INDEPENDENT — equal-primary rows
    // keep a deterministic order regardless of sort direction.
    return primary !== 0 ? primary : a.id.localeCompare(b.id, "en");
  });
}

// UI filter state over the FROZEN Session projection (§13 family) — pure, no
// daemon dep / mutation / intent. Project scoping is owned globally by the
// active-project switcher (the Shell pre-filters rows), so there's no project
// facet here — status + free text only.
export interface SessionsFilter {
  /** Free text; matched case-insensitively against name OR project name. */
  text: string;
  /** Exact status to keep, or null for "any status". */
  status: string | null;
}

/**
 * AND-compose the active facets. Both inactive → the input array unchanged
 * (identity — a cheap no-op that also keeps referential stability). The text
 * facet is trimmed, so whitespace-only is treated as no constraint.
 */
export function filterSessionRows(
  rows: SessionRowVM[],
  filter: SessionsFilter,
): SessionRowVM[] {
  const text = filter.text.trim().toLowerCase();
  if (filter.status === null && text === "") return rows;
  return rows.filter((row) => {
    if (filter.status !== null && row.status !== filter.status) return false;
    if (text !== "") {
      // OR each field independently — never join them (a joined string would match
      // a term straddling the field boundary, e.g. "<label end> <project start>").
      const inName = row.label.toLowerCase().includes(text);
      const inProject = row.projectName.toLowerCase().includes(text);
      if (!inName && !inProject) return false;
    }
    return true;
  });
}

/**
 * The distinct statuses present in the (UNFILTERED) rows, deduped + alphabetically
 * ordered (deterministic). Computed off the full set so the filter options stay
 * stable instead of collapsing as the active filter narrows the rows.
 */
export function distinctStatuses(rows: SessionRowVM[]): string[] {
  return [...new Set(rows.map((r) => r.status))].toSorted((a, b) =>
    a.localeCompare(b, "en"),
  );
}

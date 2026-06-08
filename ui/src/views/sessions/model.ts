// The Sessions table model (§11.2 dense list + §5.2 attention ordering): a pure
// derivation of the Session projection into sortable rows. Each session → a row
// enriched with its attention rank (descriptor table — single source) and its
// project name (a pure project_id→ProjectActivity.name join, §4.2 — never invented
// state). Base shape via the L1 toSessionItems mapper (no inline re-map — §8).
import type { ProjectActivityRow, SessionRow } from "../../contracts/index";
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
    const projectId = sessions[i]?.project_id;
    return {
      ...item,
      attentionRank: describeStatus(item.machine, item.status).attentionRank,
      projectName:
        projectId == null
          ? NO_PROJECT
          : // unmatched project → the raw id stays visible
            (nameByProjectId.get(projectId) ?? projectId),
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

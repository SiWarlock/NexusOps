import { useState } from "react";
import type { ProjectActivityRow, SessionRow } from "../../contracts/index";
import { StatusPill } from "../../status/StatusPill";
import { AttentionMarker } from "../../status/AttentionMarker";
import {
  buildSessionRows,
  sortSessionRows,
  naturalDir,
  DEFAULT_SORT,
  type SessionsSort,
  type SessionsSortKey,
} from "./model";

interface SessionsTableProps {
  sessions: SessionRow[];
  projects: ProjectActivityRow[];
}

const COLUMNS: { key: SessionsSortKey; label: string }[] = [
  { key: "name", label: "Name" },
  { key: "status", label: "Status" },
  { key: "attention", label: "Attention" },
  { key: "project", label: "Project" },
];

// aria-sort reflects the sort STATE (not the last click): the active column shows
// its direction; every other column is "none". So a default-sorted table is
// accurately described on first paint, before any interaction.
function ariaSortFor(
  sort: SessionsSort,
  key: SessionsSortKey,
): "ascending" | "descending" | "none" {
  if (sort.key !== key) return "none";
  return sort.dir === "asc" ? "ascending" : "descending";
}

/**
 * The Sessions list (§11.2) — a dense, sortable semantic table of every session
 * (Name / Status / Attention / Project), attention-sorted by default (§5.2) and
 * sortable by any column header (the `<button>`-in-`<th>` headers are keyboard-
 * operable; `aria-sort` tracks state). Renders ONLY the projection's sessions
 * (no invented rows — forbidden #2); reads its data through props (the Shell's
 * gateway boundary). Board / filtering / model+team columns are deferred.
 */
export function SessionsTable({ sessions, projects }: SessionsTableProps) {
  const [sort, setSort] = useState<SessionsSort>(DEFAULT_SORT);
  const rows = sortSessionRows(
    buildSessionRows(sessions, projects),
    sort.key,
    sort.dir,
  );

  // Click the active column → toggle direction; a new column → its natural dir.
  const onSort = (key: SessionsSortKey) =>
    setSort((prev) =>
      prev.key === key
        ? { key, dir: prev.dir === "asc" ? "desc" : "asc" }
        : { key, dir: naturalDir(key) },
    );

  return (
    <div className="sessions" aria-label="Sessions">
      <table className="sessions__table" data-testid="sessions-table">
        <thead>
          <tr>
            {COLUMNS.map((col) => (
              <th
                key={col.key}
                scope="col"
                data-sort-key={col.key}
                aria-sort={ariaSortFor(sort, col.key)}
              >
                <button type="button" onClick={() => onSort(col.key)}>
                  {col.label}
                </button>
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {rows.length === 0 ? (
            <tr>
              <td colSpan={COLUMNS.length} data-testid="sessions-empty">
                No sessions.
              </td>
            </tr>
          ) : (
            rows.map((row) => (
              <tr key={row.id} data-item-id={`Session:${row.id}`}>
                <td className="sessions__name">{row.label}</td>
                <td>
                  <StatusPill machine={row.machine} status={row.status} size="xs" />
                </td>
                <td>
                  <AttentionMarker rank={row.attentionRank} variant="dot" />
                </td>
                <td className="sessions__project">{row.projectName}</td>
              </tr>
            ))
          )}
        </tbody>
      </table>
    </div>
  );
}

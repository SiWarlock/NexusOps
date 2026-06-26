import { useState, type ReactNode } from "react";
import type {
  ProjectActivityRow,
  ResumeMode,
  SessionRow,
} from "../../contracts/index";
import { StatusPill } from "../../status/StatusPill";
import { AttentionMarker } from "../../status/AttentionMarker";
import { describeResumeMode } from "../../recovery/model";
import {
  buildSessionRows,
  sortSessionRows,
  filterSessionRows,
  distinctStatuses,
  naturalDir,
  DEFAULT_SORT,
  type SessionsFilter,
  type SessionsSort,
  type SessionsSortKey,
  type SessionRowVM,
} from "./model";

const NO_FILTER: SessionsFilter = { text: "", status: null };

// O-2 survival display: a session's resumed-(live) vs replayed-(relaunched)
// indicator — glyph + label (never color alone — §11.6).
function ResumeModeBadge({ mode }: { mode: ResumeMode }) {
  const desc = describeResumeMode(mode);
  return (
    <span className="sessions__resume-mode" data-resume-mode={mode}>
      <span aria-hidden="true">{desc.glyph}</span> {desc.label}
    </span>
  );
}

interface SessionsTableProps {
  sessions: SessionRow[];
  projects: ProjectActivityRow[];
  /** Optional toolbar slot (e.g. the WAVE-1 "Launch agent" control) rendered in the filters header. */
  headerActions?: ReactNode;
  /** Optional per-row action slot (e.g. the WAVE-1 "Kill" control) — a render-prop that keeps the table
   *  gateway-agnostic / purely presentational (the `headerActions` mirror, forbidden #2). When supplied,
   *  an "Actions" column is rendered and each body row gets `rowActions(row)` for its OWN row. */
  rowActions?: (row: SessionRowVM) => ReactNode;
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
export function SessionsTable({ sessions, projects, headerActions, rowActions }: SessionsTableProps) {
  const [sort, setSort] = useState<SessionsSort>(DEFAULT_SORT);
  const [filter, setFilter] = useState<SessionsFilter>(NO_FILTER);

  // Column count for the empty-state colSpan: the sort columns + the Recovery column + (when supplied)
  // the per-row Actions column.
  const totalCols = COLUMNS.length + 1 + (rowActions ? 1 : 0);

  // Pipeline: build -> filter -> sort. Status options come from the UNFILTERED set
  // (stable as the filter narrows). truly-empty (no rows at all) is distinct from
  // filtered-empty (rows exist but the filter excludes all).
  const built = buildSessionRows(sessions, projects);
  const statuses = distinctStatuses(built);
  const rows = sortSessionRows(filterSessionRows(built, filter), sort.key, sort.dir);
  const trulyEmpty = built.length === 0;
  const filteredEmpty = !trulyEmpty && rows.length === 0;

  // Click the active column → toggle direction; a new column → its natural dir.
  const onSort = (key: SessionsSortKey) =>
    setSort((prev) =>
      prev.key === key
        ? { key, dir: prev.dir === "asc" ? "desc" : "asc" }
        : { key, dir: naturalDir(key) },
    );

  return (
    <div className="sessions" aria-label="Sessions">
      <div className="sessions__filters" role="group" aria-label="Filter sessions">
        <label className="sessions__filter">
          <span className="sr-only">Filter by status</span>
          <select
            value={filter.status ?? ""}
            onChange={(e) =>
              setFilter((f) => ({ ...f, status: e.target.value || null }))
            }
          >
            <option value="">All statuses</option>
            {statuses.map((s) => (
              <option key={s} value={s}>
                {s}
              </option>
            ))}
          </select>
        </label>
        <label className="sessions__filter">
          <span className="sr-only">Search sessions</span>
          <input
            type="search"
            placeholder="Search sessions…"
            value={filter.text}
            onChange={(e) => setFilter((f) => ({ ...f, text: e.target.value }))}
          />
        </label>
        {headerActions ? (
          <div className="sessions__header-actions" style={{ marginLeft: "auto" }}>
            {headerActions}
          </div>
        ) : null}
      </div>
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
            {/* Recovery is a display-only column (the resume-mode indicator) — not
                a sort key, so a plain header (no sort button). */}
            <th scope="col">Recovery</th>
            {/* Actions is an opt-in per-row control column (the rowActions render-prop slot). */}
            {rowActions ? <th scope="col">Actions</th> : null}
          </tr>
        </thead>
        <tbody>
          {filteredEmpty ? (
            <tr>
              <td colSpan={totalCols} data-testid="sessions-filtered-empty">
                No sessions match the filters.{" "}
                <button type="button" onClick={() => setFilter(NO_FILTER)}>
                  Clear filters
                </button>
              </td>
            </tr>
          ) : rows.length === 0 ? (
            <tr>
              <td colSpan={totalCols} data-testid="sessions-empty">
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
                <td className="sessions__recovery">
                  {row.resumeMode ? <ResumeModeBadge mode={row.resumeMode} /> : "—"}
                </td>
                {rowActions ? (
                  <td className="sessions__actions">{rowActions(row)}</td>
                ) : null}
              </tr>
            ))
          )}
        </tbody>
      </table>
    </div>
  );
}

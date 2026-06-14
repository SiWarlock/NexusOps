// The projection delta-reducer (P6.8 L1, slice 052, layer C).
//
// Applies a streamed `ProjectionDelta` to the live in-memory projection rows so the cockpit
// re-renders as the daemon mutates. The local store is a READ CACHE only (forbidden #2 — the
// daemon's event log stays authoritative; we never invent or hold authoritative state). 052 lands
// the mechanism on the Session projection (the most dynamic); the other projections reuse the
// identical shape (a per-projection reducer + ShellData field) as a spread.
import type { ProjectionDelta, SessionRow } from "../contracts/index";

/**
 * Apply a Session `ProjectionDelta` to `rows`, returning a NEW array (referential change drives the
 * React re-render; the input is never mutated). `upsert` replaces the row matching `row.session_id`,
 * or appends it when absent; `remove` drops the row matching `delta.id`. A malformed delta (an
 * `upsert` with no row, a `remove` with no id) is a defensive no-op (returns `rows` unchanged) —
 * the daemon always populates them, but the cache must never crash on an unexpected frame.
 */
export function applySessionDelta(
  rows: SessionRow[],
  delta: ProjectionDelta,
): SessionRow[] {
  // Defense-in-depth: the Session subscribe connection is dedicated/single-projection, but never
  // apply a non-Session delta's row into the Session cache if one ever arrives (parse-don't-trust
  // posture — the connection's dedication is not the only guard).
  if (delta.projection !== "Session") return rows;
  if (delta.kind === "upsert") {
    const row = delta.row;
    if (!row) return rows;
    const idx = rows.findIndex((r) => r.session_id === row.session_id);
    if (idx >= 0) {
      const next = rows.slice();
      next[idx] = row;
      return next;
    }
    return [...rows, row];
  }
  // remove
  if (delta.id == null) return rows;
  return rows.filter((r) => r.session_id !== delta.id);
}

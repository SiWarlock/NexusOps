import { describe, it, expect } from "vitest";
import { applySessionDelta } from "./delta-reducer";
import type { ProjectionDelta, SessionRow } from "../contracts/index";
import { sessionPageFixture } from "../projections/fixtures/proj_session";

const baseRows = sessionPageFixture.rows as SessionRow[];

describe("applySessionDelta — live Session re-render (read cache; forbidden #2)", () => {
  it("upsert replaces an existing row by session_id (new array — referential change drives re-render)", () => {
    const target = baseRows[0]!;
    const updated: SessionRow = { ...target, title: "Renamed session" };
    const delta: ProjectionDelta = {
      projection: "Session",
      kind: "upsert",
      row: updated,
      id: target.session_id,
    };

    const next = applySessionDelta(baseRows, delta);

    expect(next).toHaveLength(baseRows.length);
    expect(next.find((r) => r.session_id === target.session_id)?.title).toBe(
      "Renamed session",
    );
    expect(next).not.toBe(baseRows); // a new array (the local store is a read cache, replaced not mutated)
    expect(baseRows[0]!.title).toBe(target.title); // the input is not mutated
  });

  it("upsert inserts a new row when the session_id is absent", () => {
    const newRow: SessionRow = { ...baseRows[0]!, session_id: "session_brand_new" };
    const delta: ProjectionDelta = {
      projection: "Session",
      kind: "upsert",
      row: newRow,
    };

    const next = applySessionDelta(baseRows, delta);

    expect(next).toHaveLength(baseRows.length + 1);
    expect(next.some((r) => r.session_id === "session_brand_new")).toBe(true);
  });

  it("remove drops the row by id", () => {
    const target = baseRows[0]!;
    const delta: ProjectionDelta = {
      projection: "Session",
      kind: "remove",
      id: target.session_id,
    };

    const next = applySessionDelta(baseRows, delta);

    expect(next).toHaveLength(baseRows.length - 1);
    expect(next.some((r) => r.session_id === target.session_id)).toBe(false);
  });

  it("an upsert with no row, or a remove with no id, is a no-op (defensive)", () => {
    expect(
      applySessionDelta(baseRows, { projection: "Session", kind: "upsert" }),
    ).toBe(baseRows);
    expect(
      applySessionDelta(baseRows, { projection: "Session", kind: "remove" }),
    ).toBe(baseRows);
  });

  it("ignores a delta for a different projection (defense-in-depth, parse-don't-trust)", () => {
    const foreign: ProjectionDelta = {
      projection: "PullRequest",
      kind: "remove",
      id: baseRows[0]!.session_id,
    };
    // a non-Session delta never touches the Session cache, even if its id collides.
    expect(applySessionDelta(baseRows, foreign)).toBe(baseRows);
  });
});

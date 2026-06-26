import { describe, it, expect } from "vitest";
import { parseProjectionPage } from "./boundary";
import { sessionPageFixture } from "../projections/fixtures/proj_session";

describe("gateway-client boundary validator (parse, don't trust)", () => {
  it("boundary_parse_accepts_valid_projection_payload", () => {
    const page = parseProjectionPage("Session", sessionPageFixture);
    expect(page.projection).toBe("Session");
    expect(page.rows.length).toBeGreaterThan(0);
  });

  it("boundary_parse_wraps_a_daemon_shaped_bare_array_into_the_page_envelope", () => {
    // The daemon serves each projection as a BARE JSON array of rows (the response
    // envelope was never frozen — the ui adapts at the boundary). The boundary
    // normalizes that wire shape into the {projection,rows} page the UI schemas
    // expect, so the SAME rows parse identically whether they arrive bare (the real
    // daemon) or pre-enveloped (the Mock fixtures) — killing the Mock-vs-real gap.
    const bareArray = sessionPageFixture.rows; // the daemon's bare-array reply
    const fromBare = parseProjectionPage("Session", bareArray);
    expect(fromBare.projection).toBe("Session");
    expect(fromBare.rows.length).toBe(sessionPageFixture.rows.length);
    // Same rows parse identically from the bare (daemon) and enveloped (Mock) shapes.
    // (The bare reply carries no cursor → absent; the enveloped fixture's is null —
    // both mean "no cursor", so compare the load-bearing rows, not that cosmetic diff.)
    expect(fromBare.rows).toEqual(parseProjectionPage("Session", sessionPageFixture).rows);
  });

  it("boundary_parse_fails_closed_on_a_bare_array_of_malformed_rows", () => {
    // A daemon-shaped bare array whose rows are malformed still fails closed.
    expect(() => parseProjectionPage("Session", [{ status: "bogus_status" }])).toThrow();
  });

  it("boundary_parse_rejects_malformed_payload", () => {
    // A row carrying a non-§5.1 status must fail closed at the boundary.
    const unknownStatus = {
      projection: "Session",
      rows: [{ session_id: "session_unknown_1", status: "bogus_status" }],
    };
    expect(() => parseProjectionPage("Session", unknownStatus)).toThrow();

    // A row missing a required field must also fail closed at the boundary.
    const missingField = {
      projection: "Session",
      rows: [{ status: "active" }],
    };
    expect(() => parseProjectionPage("Session", missingField)).toThrow();
  });

  it("boundary_parses_a_real_name_less_projectactivity_row", () => {
    // spec(§7.2) — the live cockpit-load path: the daemon serves ProjectActivity as a bare array of
    // name-LESS counter rows (proj_project_activity has no `name` col). The required-`name` shadow
    // rejected EVERY real row (the 5th Mock-vs-real trap); the relaxed shadow must accept it.
    const page = parseProjectionPage("ProjectActivity", [
      { project_id: "proj_x", active_sessions: 1, open_prs: 0, updated_at_seq: 3 },
    ]);
    expect(page.projection).toBe("ProjectActivity");
    expect(page.rows).toHaveLength(1);
    expect((page.rows[0] as { project_id: string }).project_id).toBe("proj_x");
  });
});

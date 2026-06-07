import { describe, it, expect } from "vitest";
import { parseProjectionPage } from "./boundary";
import { sessionPageFixture } from "../projections/fixtures/proj_session";

describe("gateway-client boundary validator (parse, don't trust)", () => {
  it("boundary_parse_accepts_valid_projection_payload", () => {
    const page = parseProjectionPage("Session", sessionPageFixture);
    expect(page.projection).toBe("Session");
    expect(page.rows.length).toBeGreaterThan(0);
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
});

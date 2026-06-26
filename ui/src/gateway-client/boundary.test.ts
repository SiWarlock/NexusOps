import { describe, it, expect } from "vitest";
import {
  parseProjectionPage,
  parseExecutionProfilesResult,
  BoundaryValidationError,
} from "./boundary";
import { sessionPageFixture } from "../projections/fixtures/proj_session";

const validProfilesPayload = {
  profiles: [
    {
      execution_profile_id: "ep_1",
      provider: "anthropic",
      harness: "claude_code",
      model: null,
      account_alias: null,
      status: "available",
      is_default: true,
      has_credential: true,
    },
  ],
};

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

  it("audit_trail_page_accepts_frozen_row_rejects_unknown", () => {
    // spec(§7.2/§5.0 — W2-audit un-degrade) — the daemon serves AuditTrail as a bare array of the
    // frozen 8-field AuditEventRow. The boundary must ACCEPT that real shape (so the tile stops
    // degrading) and REJECT an unknown key (parse-don't-trust [[22]] + the `.strict()` shadow).
    const row = {
      event_id: "event_1",
      seq: 10,
      project_id: "project_1",
      occurred_at: "2026-06-26T17:47:05Z",
      event_type: "session.started",
      headline: "Started session on auth-service",
      actor_label: "action_gateway",
      sensitivity: "internal",
    };
    const page = parseProjectionPage("AuditTrail", [row]);
    expect(page.projection).toBe("AuditTrail");
    expect(page.rows).toHaveLength(1);
    expect((page.rows[0] as { event_id: string }).event_id).toBe("event_1");
    // an unknown row key (e.g. the always-NULL scope_json the daemon drops) → fail closed.
    expect(() => parseProjectionPage("AuditTrail", [{ ...row, scope_json: null }])).toThrow(
      BoundaryValidationError,
    );
  });
});

describe("parseExecutionProfilesResult (§6.1 W1-prof — read-RPC result boundary)", () => {
  it("parse_execution_profiles_result_accepts_valid", () => {
    // spec(§5.0) — a valid get_execution_profiles payload → the typed GetExecutionProfilesResult,
    // consumed DIRECTLY (not parseProjectionPage — it's a read-RESULT struct, not a projection page).
    const parsed = parseExecutionProfilesResult(validProfilesPayload);
    expect(parsed.profiles).toHaveLength(1);
    expect(parsed.profiles[0]!.execution_profile_id).toBe("ep_1");
    expect(parsed.profiles[0]!.is_default).toBe(true);
  });

  it("parse_execution_profiles_result_fail_closes_on_malformed", () => {
    // spec(§5.0 parse-don't-trust) — a malformed payload → BoundaryValidationError (never reaches the
    // profile picker). Includes the §15 #4 secret-free pin: a leaked keychain_ref is rejected (.strict()).
    expect(() => parseExecutionProfilesResult({ not_profiles: true })).toThrow(
      BoundaryValidationError,
    );
    expect(() =>
      parseExecutionProfilesResult({
        profiles: [{ ...validProfilesPayload.profiles[0], keychain_ref: "kc://leak" }],
      }),
    ).toThrow(BoundaryValidationError);
  });
});

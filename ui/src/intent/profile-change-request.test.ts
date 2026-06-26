import { describe, it, expect } from "vitest";
import {
  buildProfileChangeActionRequest,
  SESSION_PROFILE_CHANGE_ACTION_TYPE,
  isProfileChangeEnabled,
} from "./profile-change-request";
import { ActionRequest } from "../contracts/index";

// The EXACT daemon `minted_id!` ULID shape (uppercase Crockford, no I/L/O/U, 26-char body,
// first char 0-7 = the 48-bit-timestamp range `Ulid::from_string` enforces, shared/src/ids.rs).
const ACT_ULID = /^act_[0-7][0-9A-HJKMNP-TV-Z]{25}$/;

describe("buildProfileChangeActionRequest (cockpit Change-profile intent — WAVE-1 W1-C-a)", () => {
  it("forms a valid session.profile_change request: session resource_ref + profile_id inputs, risk-2", () => {
    const req = buildProfileChangeActionRequest(
      { session_id: "session_fixture_1", execution_profile_id: "ep_2" },
      "2026-06-26T00:00:00Z",
    );
    expect(req.action_type).toBe(SESSION_PROFILE_CHANGE_ACTION_TYPE);
    expect(req.action_type).toBe("session.profile_change");
    // execute_profile_change keys on resource_refs.first() → SessionId::parse (session_executor.rs:207)
    expect(req.resource_refs).toEqual([{ type: "session", id: "session_fixture_1" }]);
    // inputs.execution_profile_id is REQUIRED (no default; parsed → ExecutionProfileId, :236)
    expect(req.inputs).toEqual({ execution_profile_id: "ep_2" });
    expect(req.requester_type).toBe("user");
    expect(req.status).toBe("submitted");
    // risk-2 hint (non-authoritative; catalog-authoritative → approval-gated, the §15 #8 checkpoint)
    expect(req.risk_level).toBe(2);
    expect(req.created_at).toBe("2026-06-26T00:00:00Z");
    expect(() => ActionRequest.parse(req)).not.toThrow();
  });

  // spec(§6.2/§5.2 LESSON 39) — every generic submit_action intent client-mints a canonical ULID PK
  // (the daemon trusts the wire id verbatim; an empty/colliding PK → AuditWriteFailed → precondition_stale).
  it("mints a non-empty prefixed action_request_id, distinct across calls", () => {
    const a = buildProfileChangeActionRequest(
      { session_id: "s1", execution_profile_id: "ep_1" },
      "2026-06-26T00:00:00Z",
    );
    const b = buildProfileChangeActionRequest(
      { session_id: "s1", execution_profile_id: "ep_1" },
      "2026-06-26T00:00:00Z",
    );
    expect(a.action_request_id).toMatch(ACT_ULID);
    expect(a.action_request_id).not.toBe("");
    expect(a.action_request_id).not.toBe(b.action_request_id);
  });
});

describe("isProfileChangeEnabled (the per-action gate reader — the isSessionKillEnabled mirror)", () => {
  it("reads the port's enabledProfileChange flag (structurally typed — no GatewayPort import)", () => {
    expect(isProfileChangeEnabled({ enabledProfileChange: true })).toBe(true);
    expect(isProfileChangeEnabled({ enabledProfileChange: false })).toBe(false);
  });
});

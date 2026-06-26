import { describe, it, expect } from "vitest";
import {
  buildKillSessionActionRequest,
  SESSION_KILL_ACTION_TYPE,
  isSessionKillEnabled,
} from "./kill-session-request";
import { ActionRequest } from "../contracts/index";

// The EXACT daemon `minted_id!` ULID shape (uppercase Crockford, no I/L/O/U, 26-char body,
// first char 0-7 = the 48-bit-timestamp range `Ulid::from_string` enforces, shared/src/ids.rs).
const ACT_ULID = /^act_[0-7][0-9A-HJKMNP-TV-Z]{25}$/;

describe("buildKillSessionActionRequest (cockpit Kill intent — WAVE-1 Slice B)", () => {
  it("forms a valid session.kill request: session resource_ref, empty inputs, client-minted id", () => {
    const req = buildKillSessionActionRequest(
      { session_id: "session_fixture_1" },
      "2026-06-26T00:00:00Z",
    );
    expect(req.action_type).toBe(SESSION_KILL_ACTION_TYPE);
    expect(req.action_type).toBe("session.kill");
    // execute_kill is NaturalResourceRef-keyed: req.resource_refs.first() → SessionId::parse(rref.id)
    // (session_executor.rs:279-293). The session resource_ref is the ONLY operand → inputs is empty.
    expect(req.resource_refs).toEqual([{ type: "session", id: "session_fixture_1" }]);
    expect(req.inputs).toEqual({});
    expect(req.requester_type).toBe("user");
    expect(req.status).toBe("submitted");
    expect(req.risk_level).toBe(0); // non-authoritative hint (catalog-authoritative; daemon-reconciled)
    expect(req.created_at).toBe("2026-06-26T00:00:00Z");
    // the built intent is a valid §6.2 ActionRequest (the frozen-shadow shape).
    expect(() => ActionRequest.parse(req)).not.toThrow();
  });

  // spec(§6.2/§5.2 LESSON 39) — every generic submit_action intent client-mints a canonical ULID PK
  // (the daemon trusts the wire id verbatim; only session.create mints daemon-side). An empty PK
  // collides on a 2nd same-session mutation → AuditWriteFailed → overloaded precondition_stale (ui-080).
  it("mints a non-empty prefixed action_request_id (never the empty PK)", () => {
    const req = buildKillSessionActionRequest({ session_id: "s1" }, "2026-06-26T00:00:00Z");
    expect(req.action_request_id).toMatch(ACT_ULID);
    expect(req.action_request_id).not.toBe("");
  });

  // spec(LESSON 39) — two kills produce distinct ids; a reused/empty PK collides → precondition_stale.
  it("mints distinct ids across calls", () => {
    const a = buildKillSessionActionRequest({ session_id: "s1" }, "2026-06-26T00:00:00Z");
    const b = buildKillSessionActionRequest({ session_id: "s1" }, "2026-06-26T00:00:00Z");
    expect(a.action_request_id).not.toBe(b.action_request_id);
  });
});

describe("isSessionKillEnabled (the per-action gate reader — the isPrMutationEnabled mirror)", () => {
  it("reads the port's enabledSessionKill flag (structurally typed — no GatewayPort import)", () => {
    expect(isSessionKillEnabled({ enabledSessionKill: true })).toBe(true);
    expect(isSessionKillEnabled({ enabledSessionKill: false })).toBe(false);
  });
});

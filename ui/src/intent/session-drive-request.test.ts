import { describe, it, expect } from "vitest";
import {
  buildSendMessageActionRequest,
  buildPauseActionRequest,
  buildResumeActionRequest,
  SESSION_DRIVE_ACTION_TYPES,
  isSessionControlEnabled,
} from "./session-drive-request";
import { ActionRequest } from "../contracts/index";

const ACT_ULID = /^act_[0-7][0-9A-HJKMNP-TV-Z]{25}$/;

describe("session-drive builders (WAVE-1 W1-C-b — send_message / pause / resume)", () => {
  it("build_session_drive_requests", () => {
    // spec(§6.3/§9.1 LESSON 39) — each builder forms the right action_type, session resource_ref, inputs,
    // and risk hint (send_message risk-2 FromInputs / pause risk-1 / resume risk-2), with a client-minted id.
    const send = buildSendMessageActionRequest(
      { session_id: "s1", message: "do the thing" },
      "2026-06-26T00:00:00Z",
    );
    expect(send.action_type).toBe("session.send_message");
    expect(send.resource_refs).toEqual([{ type: "session", id: "s1" }]);
    expect(send.inputs).toEqual({ message: "do the thing" });
    expect(send.risk_level).toBe(2);
    expect(send.action_request_id).toMatch(ACT_ULID);
    expect(() => ActionRequest.parse(send)).not.toThrow();

    const pause = buildPauseActionRequest({ session_id: "s1" }, "2026-06-26T00:00:00Z");
    expect(pause.action_type).toBe("session.pause");
    expect(pause.resource_refs).toEqual([{ type: "session", id: "s1" }]);
    expect(pause.inputs).toEqual({});
    expect(pause.risk_level).toBe(1);
    expect(pause.action_request_id).toMatch(ACT_ULID);
    expect(() => ActionRequest.parse(pause)).not.toThrow();

    const resume = buildResumeActionRequest({ session_id: "s1" }, "2026-06-26T00:00:00Z");
    expect(resume.action_type).toBe("session.resume");
    expect(resume.resource_refs).toEqual([{ type: "session", id: "s1" }]);
    expect(resume.inputs).toEqual({});
    expect(resume.risk_level).toBe(2);
    expect(resume.action_request_id).toMatch(ACT_ULID);
    expect(() => ActionRequest.parse(resume)).not.toThrow();
  });

  it("send_message_builder_trims_message", () => {
    // spec(§6.3) — inputs.message is .trim()'d (defense-in-depth; the daemon FromInputs rejects an empty
    // message — the control blocks empty first, the builder trims as the second line).
    const req = buildSendMessageActionRequest(
      { session_id: "s1", message: "  hi there  " },
      "2026-06-26T00:00:00Z",
    );
    expect(req.inputs).toEqual({ message: "hi there" });
  });

  it("mints_distinct_ids_across_builders_and_calls", () => {
    // spec(LESSON 39) — each generic submit_action intent client-mints a distinct ULID PK (a reused/empty
    // PK collides → precondition_stale).
    const a = buildPauseActionRequest({ session_id: "s1" }, "2026-06-26T00:00:00Z");
    const b = buildPauseActionRequest({ session_id: "s1" }, "2026-06-26T00:00:00Z");
    const c = buildResumeActionRequest({ session_id: "s1" }, "2026-06-26T00:00:00Z");
    expect(new Set([a.action_request_id, b.action_request_id, c.action_request_id]).size).toBe(3);
  });
});

describe("isSessionControlEnabled + SESSION_DRIVE_ACTION_TYPES (the Set-gate reader)", () => {
  it("the_drive_set_holds_the_three_gated_types", () => {
    expect([...SESSION_DRIVE_ACTION_TYPES].toSorted()).toEqual(
      ["session.pause", "session.resume", "session.send_message"],
    );
  });

  it("is_session_control_enabled_reads_the_port_set", () => {
    // spec(the enabledPrMutations-style Set gate) — enabled iff the action_type is in the port's set.
    const gw = { enabledSessionControls: new Set(["session.pause"]) };
    expect(isSessionControlEnabled(gw, "session.pause")).toBe(true);
    expect(isSessionControlEnabled(gw, "session.resume")).toBe(false);
  });
});

import { describe, it, expect } from "vitest";
import { buildRescanProjectActionRequest } from "./rescan-project-request";
import { ActionRequest } from "../contracts/index";

// The EXACT daemon `minted_id!` ULID shape (uppercase Crockford, no I/L/O/U, 26-char body,
// first char 0-7 = the 48-bit-timestamp range `Ulid::from_string` enforces, shared/src/ids.rs).
const ACT_ULID = /^act_[0-7][0-9A-HJKMNP-TV-Z]{25}$/;
const PROJ_ULID = /^proj_[0-7][0-9A-HJKMNP-TV-Z]{25}$/;

describe("buildRescanProjectActionRequest (cockpit add-project intent)", () => {
  it("forms a valid project.rescan request: path in inputs, NO resource_refs", () => {
    const req = buildRescanProjectActionRequest(
      { path: "/repos/auth-service" },
      "2026-06-26T00:00:00Z",
    );
    expect(req.action_type).toBe("project.rescan");
    // requires_resource_refs=false — the scan path is the target, not a resource_ref.
    expect(req.resource_refs).toEqual([]);
    expect(req.inputs).toEqual({ path: "/repos/auth-service" });
    expect(req.requester_type).toBe("user");
    expect(req.created_at).toBe("2026-06-26T00:00:00Z");
    // the built intent is a valid §6.2 ActionRequest (the frozen-shadow shape).
    expect(() => ActionRequest.parse(req)).not.toThrow();
  });

  it("trims the path so a whitespace-only pick can't become the scan target", () => {
    // the daemon fails CLOSED on a blank inputs.path; trim defense-in-depth here so a
    // padded pick never reaches the daemon as the scan target (mirrors LESSON §33).
    const req = buildRescanProjectActionRequest(
      { path: "  /repos/x  " },
      "2026-06-26T00:00:00Z",
    );
    expect(req.inputs).toEqual({ path: "/repos/x" });
  });

  // spec(§6.2/§5.2): the client supplies the id — the daemon trusts the wire `action_request_id`
  // verbatim as the `action_requests` TEXT PRIMARY KEY (only session.create mints daemon-side).
  // An empty PK derails the risk-0 auto-execute → overloaded `precondition_stale`.
  it("mints a non-empty prefixed action_request_id (never the empty PK)", () => {
    const req = buildRescanProjectActionRequest(
      { path: "/repos/x" },
      "2026-06-26T00:00:00Z",
    );
    expect(req.action_request_id).toMatch(ACT_ULID);
    expect(req.action_request_id).not.toBe("");
  });

  // spec(§6.2): the project identity rides ActionRequest.project_id (the ENVELOPE), which the
  // daemon stamps onto the ProjectRescanned envelope; the projector skips a None project_id
  // (project_registry.rs:50) → the project never registers without it.
  it("mints a non-empty prefixed project_id on the envelope (not only inputs)", () => {
    const req = buildRescanProjectActionRequest(
      { path: "/repos/x" },
      "2026-06-26T00:00:00Z",
    );
    expect(req.project_id).toMatch(PROJ_ULID);
  });

  // spec(§6.2): two Adds produce distinct ids — a reused/empty PK collides →
  // AuditWriteFailed→precondition_stale (the 2nd-Add bug); each Add is a fresh action + project.
  it("mints distinct action_request_id AND project_id across calls", () => {
    const a = buildRescanProjectActionRequest({ path: "/r/a" }, "2026-06-26T00:00:00Z");
    const b = buildRescanProjectActionRequest({ path: "/r/b" }, "2026-06-26T00:00:00Z");
    expect(a.action_request_id).not.toBe(b.action_request_id);
    expect(a.project_id).not.toBe(b.project_id);
  });

  // regression guard (LESSON §33 + the already-correct fields): the mint change must not
  // disturb the clean fields. Green-at-RED by design (guards the GREEN edit).
  it("preserves the existing clean fields", () => {
    const req = buildRescanProjectActionRequest(
      { path: "/repos/x" },
      "2026-06-26T00:00:00Z",
    );
    expect(req.resource_refs).toEqual([]);
    expect(req.risk_level).toBe(0);
    expect(req.requester_type).toBe("user");
    expect(req.action_type).toBe("project.rescan");
    expect(req.status).toBe("submitted");
  });
});

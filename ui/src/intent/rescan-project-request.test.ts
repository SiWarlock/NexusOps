import { describe, it, expect } from "vitest";
import { buildRescanProjectActionRequest } from "./rescan-project-request";
import { ActionRequest } from "../contracts/index";

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
    expect(req.action_request_id).toBe(""); // daemon mints
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
});

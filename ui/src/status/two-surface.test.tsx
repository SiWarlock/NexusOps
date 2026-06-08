// @vitest-environment jsdom
import { describe, it, expect, afterEach } from "vitest";
import { cleanup, render } from "@testing-library/react";
import { StatusPill } from "./StatusPill";

afterEach(cleanup);

describe("Approval vs ActionRequest — two distinct surfaces (R-5)", () => {
  it("approval_and_action_request_render_two_distinct_surfaces", () => {
    // the same lifecycle word (awaiting_approval) on the two split axes
    const { container: approval } = render(
      <StatusPill machine="Approval" status="awaiting_approval" />,
    );
    const { container: action } = render(
      <StatusPill machine="ActionRequest" status="awaiting_approval" />,
    );
    const approvalSurface = approval.querySelector("[data-machine]")!;
    const actionSurface = action.querySelector("[data-machine]")!;

    // tagged as two different machines (decision axis vs execution axis)
    expect(approvalSurface.getAttribute("data-machine")).toBe("Approval");
    expect(actionSurface.getAttribute("data-machine")).toBe("ActionRequest");

    // and VISIBLY distinct (not collapsed into one identical pill)
    expect(approvalSurface.textContent).not.toBe(actionSurface.textContent);
    expect(approvalSurface.textContent).toContain("Approval");
    expect(actionSurface.textContent).toContain("Action");
  });
});

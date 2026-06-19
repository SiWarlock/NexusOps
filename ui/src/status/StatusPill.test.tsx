// @vitest-environment jsdom
import { describe, it, expect, afterEach } from "vitest";
import { cleanup, render } from "@testing-library/react";
import { StatusPill } from "./StatusPill";
import { describeStatus } from "./descriptors";
import { validators } from "../contracts/index";

afterEach(cleanup);

const STATUS_MACHINES = [
  "Session",
  "Task",
  "PullRequest",
  "WorkflowInstance",
  "ProjectBrain",
  "Approval",
  "ActionRequest",
  "AgentTeam",
  "WorktreeGit",
  "WorktreeOverlay",
] as const;

// The UI machine identifier is UI-render-policy naming, decoupled from the frozen
// enum `$def` name. The 0.19.0 Gateway freeze renamed the two R-5 value-sets; the
// validators record is `$def`-keyed, so the two diverging machines alias here.
const VALIDATOR_KEY: Record<string, string> = {
  Approval: "ApprovalStatus",
  ActionRequest: "ActionRequestStatus",
};

describe("StatusPill wrapper", () => {
  it("status_pill_renders_every_frozen_state_four_channel", () => {
    // [load-bearing] every frozen §5.1 state renders glyph + text label (the two
    // jsdom-testable non-color channels) — never color-only, never empty/idle.
    for (const machine of STATUS_MACHINES) {
      const validator = validators[VALIDATOR_KEY[machine] ?? machine];
      expect(validator, `STATUS_MACHINES/validators out of sync: ${machine}`).toBeDefined();
      for (const status of validator!.options as readonly string[]) {
        const { container, unmount } = render(
          <StatusPill machine={machine} status={status} />,
        );
        const pill = container.querySelector('[role="status"]');
        expect(pill, `${machine}/${status} pill`).not.toBeNull();
        const glyph = pill!.querySelector("[aria-hidden]");
        expect(glyph, `${machine}/${status} glyph`).not.toBeNull();
        expect(
          glyph!.textContent!.length,
          `${machine}/${status} glyph char`,
        ).toBeGreaterThan(0);
        const label = describeStatus(machine, status).label;
        expect(pill!.textContent, `${machine}/${status} label`).toContain(label);
        unmount();
      }
    }
  });

  it("status_pill_uses_descriptor_label_and_visualkind", () => {
    const { container } = render(
      <StatusPill machine="Session" status="waiting_on_permission" />,
    );
    const pill = container.querySelector('[role="status"]')!;
    // humanized descriptor label, not the raw snake_case status
    expect(pill.textContent).toContain("Waiting on permission");
    expect(pill.textContent).not.toContain("waiting_on_permission");
  });

  it("status_pill_unknown_renders_visible_not_kit_idle", () => {
    // §11.3 through the render path: an out-of-table status renders a VISIBLE
    // (degraded) pill, not the kit's silent idle fallback (STATUS[x]||STATUS.idle).
    const { container } = render(
      <StatusPill machine="Session" status="totally_bogus" />,
    );
    const pill = container.querySelector('[role="status"]')!;
    const glyph = pill.querySelector("[aria-hidden]")!;
    expect(pill.textContent).toContain("Unknown");
    expect(glyph.textContent!.length).toBeGreaterThan(0);
    // the load-bearing signal: glyph is NOT the kit idle glyph "○" (label is
    // overridden either way, so the glyph is what proves the unknown→degraded guard)
    expect(glyph.textContent).not.toBe("○");
    expect(pill.textContent).not.toContain("Idle");
  });
});

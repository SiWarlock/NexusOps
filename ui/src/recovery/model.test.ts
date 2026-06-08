import { describe, it, expect } from "vitest";
import { describeRecovery, describeResumeMode } from "./model";

describe("recovery model", () => {
  it("recovery_state_to_banner_descriptor", () => {
    const recovering = describeRecovery("recovering");
    expect(recovering).toMatchObject({ kind: "recovering", visible: true, restartApplies: false });
    expect(recovering.message).toContain("Recovering"); // pins intent, not just non-empty

    const recovered = describeRecovery("recovered");
    expect(recovered).toMatchObject({ kind: "recovered", visible: false, restartApplies: false });

    // recovery_failed → the (parked) restart affordance applies
    const failed = describeRecovery("recovery_failed");
    expect(failed).toMatchObject({ kind: "recovery_failed", visible: true, restartApplies: true });
  });

  it("recovered_is_non_intrusive", () => {
    // recovered (or absent recovery) → no blocking banner
    expect(describeRecovery("recovered").visible).toBe(false);
    // recovering / recovery_failed ARE surfaced
    expect(describeRecovery("recovering").visible).toBe(true);
    expect(describeRecovery("recovery_failed").visible).toBe(true);
  });

  it("resume_mode_to_indicator", () => {
    const resumed = describeResumeMode("resumed");
    const replayed = describeResumeMode("replayed");
    // resumed = live, replayed = relaunched — distinct glyph + label (never color alone)
    expect(resumed.label).toContain("Resumed");
    expect(replayed.label).toContain("Replayed");
    expect(resumed.glyph).toBeTruthy();
    expect(replayed.glyph).toBeTruthy();
    expect(resumed.glyph).not.toBe(replayed.glyph);
    expect(resumed.label).not.toBe(replayed.label);
  });
});

import { describe, it, expect } from "vitest";
import { describeConflict } from "./model";
import { fencingConflictFixture } from "./fixtures";

describe("safety model — fencing/hard-conflict (§17, safety #6)", () => {
  it("conflict_to_card_descriptor", () => {
    const desc = describeConflict(fencingConflictFixture);
    // affected refs surfaced verbatim — never invented (forbidden #2)
    expect(desc.actionRequestId).toBe(fencingConflictFixture.action_request_id);
    expect(desc.sessionId).toBe(fencingConflictFixture.session_id);
    expect(desc.reason).toBe("fencing_conflict");
    expect(desc.summary).toBe(fencingConflictFixture.summary);
    // load-bearing #6: the descriptor states manual resolution, never auto-resolved
    expect(desc.message).toMatch(/never auto-resolved/i);
    expect(desc.message).toMatch(/manual resolution/i);
    // the resolution is a PARKED daemon-1.5 intent (rendered disabled-but-present)
    expect(desc.resolutionParked).toBe(true);
    // never-color-alone (§11.6): a non-color channel — glyph + label + severity
    expect(desc.glyph).toBeTruthy();
    expect(desc.label).toBeTruthy();
    expect(desc.severity).toBe("critical");
    // #6: the descriptor exposes EXACTLY these keys — no auto-resolve path can
    // slip in under any name (stronger than a substring scan for "auto").
    expect(new Set(Object.keys(desc))).toEqual(
      new Set([
        "reason",
        "actionRequestId",
        "sessionId",
        "summary",
        "message",
        "resolutionParked",
        "glyph",
        "label",
        "severity",
      ]),
    );
  });
});

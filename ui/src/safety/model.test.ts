import { describe, it, expect } from "vitest";
import { describeConflict, describeAuditIntegrity } from "./model";
import { fencingConflictFixture } from "./fixtures";
import { ActionRequestStatus, AuditOutcomeStatus, AuditIntegrityKind } from "../contracts/index";

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

describe("safety model — audit-integrity (§15/§17, safety #5)", () => {
  it("audit_integrity_reuses_frozen_action_status", () => {
    // the two action-outcome treatments REUSE the frozen ActionRequestStatus enum —
    // never a provisional re-declaration (Lesson §2 / no-drift)
    expect(ActionRequestStatus.options).toContain("partially_succeeded");
    expect(ActionRequestStatus.options).toContain("rollback_failed");
    // AuditOutcomeStatus is a strict subset of the frozen enum (drift-pinned)
    for (const v of AuditOutcomeStatus.options) {
      expect(ActionRequestStatus.options).toContain(v);
    }
    const partial = describeAuditIntegrity({
      source: "action_status",
      status: "partially_succeeded",
    });
    const rollback = describeAuditIntegrity({
      source: "action_status",
      status: "rollback_failed",
    });
    expect(partial.treatment).toBe("partially_succeeded");
    expect(rollback.treatment).toBe("rollback_failed");
    expect(partial.label).toBeTruthy();
    expect(rollback.label).toBeTruthy();
    // severity is load-bearing: partially_succeeded is the lone `warning`, a
    // rollback failure is `critical` — pin both so a typo can't flip them
    expect(partial.severity).toBe("warning");
    expect(rollback.severity).toBe("critical");
    // glyph tracks severity by construction (never color alone — §11.6)
    expect(partial.glyph).toBe("⚠");
    expect(rollback.glyph).toBe("⛔");
  });

  it("audit_integrity_provisional_only_for_net_new", () => {
    // derived from the enum `.options` (drift-proof — a future kind is covered here)
    const kinds = AuditIntegrityKind.options;
    const descs = kinds.map((kind) =>
      describeAuditIntegrity({ source: "integrity", kind }),
    );
    // each net-new state → a distinct non-color descriptor (glyph + label + severity)
    for (const d of descs) {
      expect(d.glyph).toBeTruthy();
      expect(d.label).toBeTruthy();
      expect(d.severity).toBeTruthy();
      expect(d.message).toBeTruthy();
      // glyph tracks severity — never under-signal a critical with a warning glyph
      expect(d.glyph).toBe(d.severity === "critical" ? "⛔" : "⚠");
    }
    // distinct labels — a real text channel per state (never color alone, §11.6)
    const labels = descs.map((d) => d.label);
    expect(new Set(labels).size).toBe(labels.length);
    // the net-new kinds are NOT frozen ActionRequestStatus values — provisional only
    for (const kind of kinds) {
      expect(ActionRequestStatus.options).not.toContain(kind);
    }
  });
});

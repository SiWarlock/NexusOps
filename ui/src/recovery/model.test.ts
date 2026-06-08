import { describe, it, expect } from "vitest";
import {
  describeRecovery,
  describeResumeMode,
  resumeModesBySessionId,
} from "./model";
import type { SessionRow } from "../contracts/index";

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

describe("resumeModesBySessionId (id-keyed side map)", () => {
  const sessions: SessionRow[] = [
    { session_id: "a", status: "idle", title: "A", resume_mode: "replayed" },
    { session_id: "b", status: "idle", title: "B" }, // fresh-started — no resume_mode
    { session_id: "c", status: "idle", title: "C", resume_mode: "resumed" },
  ];

  it("resume_map_includes_only_sessions_with_a_mode", () => {
    const map = resumeModesBySessionId(sessions);
    // only resumed/replayed sessions get an entry; a fresh-started session is absent
    expect("a" in map).toBe(true);
    expect("c" in map).toBe(true);
    expect("b" in map).toBe(false);
  });

  it("resume_map_keys_by_session_id", () => {
    const map = resumeModesBySessionId(sessions);
    // keyed by session_id; value is the exact ResumeMode (the §8 side-map mechanism)
    expect(map).toEqual({ a: "replayed", c: "resumed" });
  });

  it("resume_map_empty_when_none", () => {
    // zero sessions, and sessions with no mode at all → {} (the no-indicator state)
    expect(resumeModesBySessionId([])).toEqual({});
    expect(
      resumeModesBySessionId([{ session_id: "x", status: "idle", title: "X" }]),
    ).toEqual({});
  });
});

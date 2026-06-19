import { describe, it, expect } from "vitest";
import { Session } from "../../contracts/index";
import {
  ENDED_SESSION_STATUSES,
  LIVE_SESSION_STATUSES,
} from "./session-lifecycle";

describe("session lifecycle partition (§5.1 / §9.1)", () => {
  it("ended_and_live_partition_covers_every_frozen_session_status", () => {
    // spec(§5.1) — Lesson §5 completeness drift-pin: a NEW daemon Session status must
    // be consciously classified ended-or-live here. If the daemon adds one, this
    // fails loudly (it's in neither set) — far better than a silent fall-through to
    // "live", which would render a live terminal for an already-ended session (#9).
    const classified = [
      ...ENDED_SESSION_STATUSES,
      ...LIVE_SESSION_STATUSES,
    ].toSorted();
    expect(classified).toEqual([...Session.options].toSorted());
  });

  it("ended_and_live_are_disjoint", () => {
    for (const s of ENDED_SESSION_STATUSES) {
      expect(LIVE_SESSION_STATUSES.has(s), `"${s}" is in BOTH sets`).toBe(false);
    }
  });
});

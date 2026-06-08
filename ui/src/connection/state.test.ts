import { describe, it, expect } from "vitest";
import { canTransition, transition } from "./state";

describe("connection state machine", () => {
  it("connection_state_legal_transitions", () => {
    // initial-connect paths from the fail-safe initial state
    expect(canTransition("connecting", "connected")).toBe(true);
    expect(canTransition("connecting", "disconnected")).toBe(true);
    expect(transition("connecting", "connected")).toBe("connected");

    // connected → disconnected → reconnecting → connected (the recovery path)
    let s = transition("connected", "disconnected");
    s = transition(s, "reconnecting");
    s = transition(s, "connected");
    expect(s).toBe("connected");

    // illegal jump is rejected (cannot reconnect before ever connecting)
    expect(canTransition("connecting", "reconnecting")).toBe(false);
    expect(() => transition("connecting", "reconnecting")).toThrow(
      /illegal connection transition/,
    );
  });
});

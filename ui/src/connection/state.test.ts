import { describe, it, expect } from "vitest";
import { canTransition, transition, worstOfConnection } from "./state";

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

  // ui-059 — the per-stream connection aggregate (the load-bearing §11.1 fail-safe input). A direct
  // pin on the pure severity ordering so a typo in the severity table is caught HERE (the port tests
  // only observe the aggregate transitively via getConnectionState).
  it("worst_of_connection_severity_ordering", () => {
    // disconnected dominates everything; reconnecting dominates connecting/connected; etc.
    expect(worstOfConnection(["connected", "disconnected"])).toBe("disconnected");
    expect(worstOfConnection(["disconnected", "connected"])).toBe("disconnected");
    expect(worstOfConnection(["connected", "reconnecting"])).toBe("reconnecting");
    expect(worstOfConnection(["reconnecting", "disconnected"])).toBe("disconnected");
    expect(worstOfConnection(["connecting", "connected"])).toBe("connecting");
    expect(worstOfConnection(["reconnecting", "connecting"])).toBe("reconnecting");
    // the load-bearing property: any non-connected member ⇒ a non-connected aggregate (canSubmitIntent
    // fail-safe FALSE while ANY stream is degraded).
    expect(worstOfConnection(["connected", "connected"])).toBe("connected");
    expect(worstOfConnection(["connected", "reconnecting", "connected"])).not.toBe(
      "connected",
    );
    // empty set ⇒ null (no stream reported yet → the caller leaves the connection as-is).
    expect(worstOfConnection([])).toBeNull();
  });
});

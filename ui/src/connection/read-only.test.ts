import { describe, it, expect } from "vitest";
import { canSubmitIntent, deriveReadOnly, INITIAL_STATUS } from "./read-only";

describe("read-only gate", () => {
  it("disconnected_or_reconnecting_is_read_only", () => {
    expect(deriveReadOnly({ connection: "disconnected", version: "compatible" })).toBe(true);
    expect(deriveReadOnly({ connection: "reconnecting", version: "compatible" })).toBe(true);
    expect(deriveReadOnly({ connection: "connected", version: "compatible" })).toBe(false);
  });

  it("can_submit_intent_false_in_read_only", () => {
    // [safety pin] no intent submission while degraded
    expect(canSubmitIntent({ connection: "disconnected", version: "compatible" })).toBe(false);
    expect(canSubmitIntent({ connection: "reconnecting", version: "compatible" })).toBe(false);
    expect(canSubmitIntent({ connection: "connected", version: "compatible" })).toBe(true);
  });

  it("can_submit_intent_fail_safe_false_on_unknown", () => {
    // [safety pin — load-bearing] before the first successful handshake, NEVER offer a mutation.
    expect(canSubmitIntent(INITIAL_STATUS)).toBe(false);
    expect(canSubmitIntent({ connection: "connecting", version: "unknown" })).toBe(false);
    // connected but version not yet confirmed → still fail-safe FALSE
    expect(canSubmitIntent({ connection: "connected", version: "unknown" })).toBe(false);
  });
});

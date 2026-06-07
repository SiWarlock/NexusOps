import { describe, it, expect } from "vitest";
import {
  checkVersionCompat,
  deriveDegradedState,
  SUPPORTED_PROTOCOL_RANGE,
} from "./version";

describe("version compatibility", () => {
  it("version_skew_out_of_range_requires_update", () => {
    expect(
      checkVersionCompat({ protocol_version: SUPPORTED_PROTOCOL_RANGE.max + 1 }),
    ).toBe("update_required");
    expect(
      checkVersionCompat({ protocol_version: SUPPORTED_PROTOCOL_RANGE.min - 1 }),
    ).toBe("update_required");
  });

  it("version_in_range_is_compatible", () => {
    expect(
      checkVersionCompat({ protocol_version: SUPPORTED_PROTOCOL_RANGE.min }),
    ).toBe("compatible");
    expect(
      checkVersionCompat({ protocol_version: SUPPORTED_PROTOCOL_RANGE.max }),
    ).toBe("compatible");
  });

  it("update_required_precedes_reconnecting", () => {
    // [precedence pin] a version mismatch is not Retry-able → it wins over a dropped connection.
    expect(deriveDegradedState("reconnecting", "update_required")).toBe(
      "update_required",
    );
    expect(deriveDegradedState("disconnected", "update_required")).toBe(
      "update_required",
    );
    // sanity: with a compatible version, the connection state drives the degraded state
    expect(deriveDegradedState("reconnecting", "compatible")).toBe("reconnecting");
  });

  it("derive_degraded_state_connected_but_unconfirmed_version_is_ok", () => {
    // Pre-handshake window: connected but version not yet confirmed → no banner
    // ("ok"), while canSubmitIntent stays fail-safe FALSE (pinned in
    // read-only.test). Intentional for now; a "checking" banner variant is a
    // 6.3+ follow-up. Unreachable through Shell (it mounts after caps resolve).
    expect(deriveDegradedState("connected", "unknown")).toBe("ok");
    expect(deriveDegradedState("connecting", "unknown")).toBe("reconnecting");
  });
});

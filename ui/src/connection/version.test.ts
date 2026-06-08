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

  it("connected_unknown_is_checking", () => {
    // Post-connect, pre-version-confirm handshake window: connected but version
    // not yet confirmed → a non-intrusive "checking" banner (was "ok" — the
    // silent-read-only gap, §11.4). canSubmitIntent stays fail-safe FALSE
    // (pinned in read-only.test) — this only EXPLAINS the existing read-only.
    expect(deriveDegradedState("connected", "unknown")).toBe("checking");
  });

  it("checking_precedence", () => {
    // checking is the LOWEST-severity degraded state — only the connected+unknown
    // window; everything more severe wins (§16 precedence).
    expect(deriveDegradedState("connected", "update_required")).toBe("update_required");
    expect(deriveDegradedState("disconnected", "unknown")).toBe("disconnected");
    // connecting = pre-handshake (transport not yet up) → reconnecting, NOT checking
    expect(deriveDegradedState("connecting", "unknown")).toBe("reconnecting");
    // reconnecting beats checking too — a dropped/recovering transport wins over
    // the version-unknown window (pins the guard ordering against inversion)
    expect(deriveDegradedState("reconnecting", "unknown")).toBe("reconnecting");
    // handshake resolved compatible → ok (no banner)
    expect(deriveDegradedState("connected", "compatible")).toBe("ok");
  });
});

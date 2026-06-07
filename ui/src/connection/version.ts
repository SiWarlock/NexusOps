// Version-skew handling (§6.4 HelloAck/VersionSkewError; §16 compatibility matrix).
import type { ConnectionState } from "./state";

export type VersionCompat = "compatible" | "update_required" | "unknown";

// The protocol_version range this UI build supports. Pinned here because §6.4/§16
// state the handshake carries protocol_version but don't yet fix a numeric range
// (flagged to the orchestrator). MVP: a single supported version.
export const SUPPORTED_PROTOCOL_RANGE = { min: 1, max: 1 } as const;

/**
 * Verdict on a handshake's protocol_version against the supported range. Only
 * ever returns "compatible" | "update_required" — the third VersionCompat value
 * "unknown" is the pre-handshake sentinel (INITIAL_STATUS), never emitted here.
 */
export function checkVersionCompat(
  capabilities: { protocol_version: number },
  range: { min: number; max: number } = SUPPORTED_PROTOCOL_RANGE,
): "compatible" | "update_required" {
  const v = capabilities.protocol_version;
  return v >= range.min && v <= range.max ? "compatible" : "update_required";
}

export type DegradedState =
  | "ok"
  | "update_required"
  | "disconnected"
  | "reconnecting";

/**
 * The single degraded-state derivation driving the banner variant. Precedence:
 * update_required > disconnected/reconnecting > ok. A version mismatch is NOT
 * Retry-able (Retry can't fix skew), so it wins over a dropped connection (§16).
 */
export function deriveDegradedState(
  connection: ConnectionState,
  version: VersionCompat,
): DegradedState {
  if (version === "update_required") return "update_required";
  if (connection === "disconnected") return "disconnected";
  if (connection === "reconnecting" || connection === "connecting") {
    return "reconnecting";
  }
  return "ok";
}

// The daemon-connection state machine (transport liveness — DISTINCT from
// LocalRunner health, §11.4). Owned by the gateway-client; the real
// UdsGatewayPort drives it from the socket, the MockGatewayPort simulates it.
export type ConnectionState =
  | "connecting" // initial — no successful handshake yet
  | "connected"
  | "reconnecting"
  | "disconnected";

export const INITIAL_CONNECTION_STATE: ConnectionState = "connecting";

// Legal transitions. `connecting` is the pre-handshake initial; you cannot jump
// to `reconnecting` before ever connecting.
const LEGAL: Record<ConnectionState, readonly ConnectionState[]> = {
  connecting: ["connected", "disconnected"],
  connected: ["disconnected", "reconnecting"],
  disconnected: ["reconnecting", "connected"],
  reconnecting: ["connected", "disconnected"],
};

export function canTransition(
  from: ConnectionState,
  to: ConnectionState,
): boolean {
  return LEGAL[from].includes(to);
}

export function transition(
  from: ConnectionState,
  to: ConnectionState,
): ConnectionState {
  if (!canTransition(from, to)) {
    throw new Error(`illegal connection transition: ${from} -> ${to}`);
  }
  return to;
}

// Worst-of precedence over a set of per-stream connection states (ui-059 — the N-stream aggregate the
// port drives the global connection to). `disconnected` > `reconnecting` > `connecting` > `connected`:
// any non-connected stream keeps the aggregate non-connected, so canSubmitIntent (which reads the
// global) stays fail-safe FALSE while ANY stream is degraded (§11.1). Returns null for an empty set
// (no stream reported yet → the caller leaves the connection as-is; the mount load-reads own the
// initial connecting→connected). The disconnected-vs-reconnecting label is the only cosmetic part.
// NOTE: a subscribe supervisor only ever reports {disconnected, reconnecting, connected}; `connecting`
// is included in the table for TOTALITY over the enum (and ranks non-connected, so it's still fail-safe).
const CONNECTION_SEVERITY: Record<ConnectionState, number> = {
  disconnected: 3,
  reconnecting: 2,
  connecting: 1,
  connected: 0,
};

export function worstOfConnection(
  states: readonly ConnectionState[],
): ConnectionState | null {
  let worst: ConnectionState | null = null;
  for (const s of states) {
    if (worst === null || CONNECTION_SEVERITY[s] > CONNECTION_SEVERITY[worst]) {
      worst = s;
    }
  }
  return worst;
}

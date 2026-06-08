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

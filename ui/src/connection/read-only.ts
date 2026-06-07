// The global READ-ONLY gate (§11.4) — DEFENSE-IN-DEPTH, never the sole guard.
//
// The load-bearing INV-SEC-1 enforcement is daemon-side (the Action Gateway
// rejects any mutation regardless of UI state). This gate is the UX half: don't
// OFFER a mutation when the daemon is unreachable or version-incompatible. It is
// FAIL-SAFE — canSubmitIntent is true ONLY when we have positively confirmed a
// connected + version-compatible daemon; any unknown/initial state → false.
//
// Provider built with createElement (not JSX) so this stays a pure .ts predicate
// module that non-React code can import freely.
import { createContext, createElement, useContext, type ReactNode } from "react";
import type { ConnectionState } from "./state";
import type { VersionCompat } from "./version";

export interface ConnectionStatus {
  connection: ConnectionState;
  version: VersionCompat;
}

/** The fail-safe starting point: not yet handshaked → read-only. */
export const INITIAL_STATUS: ConnectionStatus = {
  connection: "connecting",
  version: "unknown",
};

/** Read-only unless we have positively confirmed connected + compatible. */
export function deriveReadOnly(status: ConnectionStatus): boolean {
  return !(status.connection === "connected" && status.version === "compatible");
}

/** The single gate every intent-submitting control consults. Fail-safe FALSE. */
export function canSubmitIntent(status: ConnectionStatus): boolean {
  return !deriveReadOnly(status);
}

const ReadOnlyContext = createContext<ConnectionStatus>(INITIAL_STATUS);

export function ReadOnlyProvider(props: {
  value: ConnectionStatus;
  children: ReactNode;
}) {
  return createElement(
    ReadOnlyContext.Provider,
    { value: props.value },
    props.children,
  );
}

export function useConnectionStatus(): ConnectionStatus {
  return useContext(ReadOnlyContext);
}

export function useReadOnly(): boolean {
  return deriveReadOnly(useContext(ReadOnlyContext));
}

export function useCanSubmitIntent(): boolean {
  return canSubmitIntent(useContext(ReadOnlyContext));
}

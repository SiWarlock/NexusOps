import type { ConnectionState } from "../connection/state";
import { ConnectionIndicator } from "../connection/ConnectionIndicator";

/**
 * Bottom status bar. Mounts the daemon-connection indicator (connected /
 * reconnecting / disconnected — distinct from LocalRunner health, §11.4) in the
 * slot 6.1b reserved.
 */
export function StatusBar({ connection }: { connection: ConnectionState }) {
  return (
    <footer
      className="status-bar"
      aria-label="Status bar"
      style={{ display: "flex", gap: 8 }}
    >
      <span className="status-bar__slot" data-slot="connection-indicator">
        <ConnectionIndicator state={connection} />
      </span>
    </footer>
  );
}

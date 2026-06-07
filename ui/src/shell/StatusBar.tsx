/**
 * Bottom status bar region. 6.1b reserves the slot for the daemon-connection
 * indicator (connected / reconnecting / disconnected, distinct from LocalRunner
 * health); that indicator + the global read-only degraded banner land in 6.1c.
 */
export function StatusBar() {
  return (
    <footer
      className="status-bar"
      aria-label="Status bar"
      style={{ display: "flex", gap: 8 }}
    >
      <span className="status-bar__slot" data-slot="connection-indicator" />
    </footer>
  );
}

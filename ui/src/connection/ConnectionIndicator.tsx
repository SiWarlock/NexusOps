import type { ConnectionState } from "./state";

// Four-channel presentation (§11.1 never-color-alone): color (via class) + glyph
// + text label. The daemon-connection (transport) indicator — distinct from
// LocalRunner health (§11.4).
const PRESENTATION: Record<ConnectionState, { glyph: string; label: string }> = {
  connecting: { glyph: "◌", label: "Connecting" },
  connected: { glyph: "●", label: "Connected" },
  reconnecting: { glyph: "◐", label: "Reconnecting" },
  disconnected: { glyph: "○", label: "Disconnected" },
};

export function ConnectionIndicator({ state }: { state: ConnectionState }) {
  const { glyph, label } = PRESENTATION[state];
  return (
    <span
      className={`conn-indicator conn-indicator--${state}`}
      role="status"
      data-state={state}
    >
      <span className="conn-indicator__glyph" aria-hidden="true">
        {glyph}
      </span>
      <span className="conn-indicator__label">{label}</span>
    </span>
  );
}

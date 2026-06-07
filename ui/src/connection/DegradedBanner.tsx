import type { DegradedState } from "./version";

/**
 * The global degraded-mode banner (§11.4). Renders only when degraded. The
 * Retry/Repair affordances stay enabled while disconnected — they're transport
 * actions, NOT gated intents. The update-required variant is distinct: a version
 * mismatch is not Retry-able (§16 update/relaunch, not retry), so it offers only
 * Repair/Update, never a plain transport Retry.
 */
export function DegradedBanner({
  degraded,
  onRetry,
  onRepair,
}: {
  degraded: DegradedState;
  onRetry: () => void;
  onRepair: () => void;
}) {
  if (degraded === "ok") return null;

  if (degraded === "update_required") {
    return (
      <div
        role="alert"
        className="degraded-banner degraded-banner--update"
        data-degraded="update_required"
      >
        <span className="degraded-banner__msg">
          Update required — the daemon’s protocol version is incompatible with
          this app. Retrying won’t help; update or relaunch.
        </span>
        <button
          type="button"
          className="degraded-banner__repair"
          onClick={onRepair}
        >
          Repair / Update
        </button>
      </div>
    );
  }

  const message =
    degraded === "reconnecting"
      ? "Reconnecting to the daemon — read-only until the connection is restored."
      : "The daemon is unreachable — read-only mode. Mutations are disabled.";

  return (
    <div role="alert" className="degraded-banner" data-degraded={degraded}>
      <span className="degraded-banner__msg">{message}</span>
      <button
        type="button"
        className="degraded-banner__retry"
        onClick={onRetry}
      >
        Retry
      </button>
      <button
        type="button"
        className="degraded-banner__repair"
        onClick={onRepair}
      >
        Repair
      </button>
    </div>
  );
}

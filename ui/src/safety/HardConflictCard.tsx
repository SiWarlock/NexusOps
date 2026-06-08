import type { FencingConflict } from "../contracts/index";
import { describeConflict } from "./model";
import { useCanSubmitIntent } from "../connection/read-only";

/**
 * The §17 fencing / hard-conflict card (safety #6) — a DISTINCT surface from the
 * transport DegradedBanner (6.1c) and the session-survival RecoveryBanner (6.4d):
 * three distinct concerns, never conflated. A stale-token / lease-expiry mutation
 * was REJECTED (`ActionFailed(fencing_conflict)`); the card surfaces it and is
 * NEVER auto-resolved (#6) — it offers no auto-resolve path at all.
 *
 * The manual-resolution control is rendered disabled/parked: the resolution
 * intent is daemon-1.5 (the single mutator / INV-SEC-1 lives daemon-side). The
 * `canSubmitIntent` gate is threaded so it's in place when the intent wires, but
 * the control stays disabled while parked regardless. DISPLAY only; renders
 * nothing when there is no conflict (non-intrusive, like 6.4d `recovered`).
 */
export function HardConflictCard({ conflict }: { conflict: FencingConflict | null }) {
  const canSubmitIntent = useCanSubmitIntent();
  if (!conflict) return null;
  const desc = describeConflict(conflict);

  return (
    <div
      className="hard-conflict-card"
      role="alert"
      data-testid="hard-conflict-card"
      data-conflict-reason={desc.reason}
      data-severity={desc.severity}
    >
      <span className="hard-conflict-card__glyph" aria-hidden="true">
        {desc.glyph}
      </span>
      <span className="hard-conflict-card__label">{desc.label}</span>
      <span
        className="hard-conflict-card__action"
        data-action-request-id={desc.actionRequestId}
      >
        {desc.actionRequestId}
      </span>
      {desc.sessionId ? (
        <span
          className="hard-conflict-card__session"
          data-session-id={desc.sessionId}
        >
          {desc.sessionId}
        </span>
      ) : null}
      <p className="hard-conflict-card__summary">{desc.summary}</p>
      <p className="hard-conflict-card__never-auto-resolved">{desc.message}</p>
      <button
        type="button"
        className="hard-conflict-card__resolve"
        data-testid="conflict-resolve"
        // Parked dominates today: `resolutionParked` is always true while the
        // resolution intent is deferred (daemon-1.5), so the control is disabled
        // regardless of connection. `canSubmitIntent` is pre-threaded so the
        // fail-safe read-only gate (forbidden #6) is already in place for when
        // the intent wires — at which point `resolutionParked` becomes false.
        disabled={desc.resolutionParked || !canSubmitIntent}
        title="Resolve conflict — manual resolution wires with the daemon (daemon-1.5); never auto-resolved."
      >
        Resolve…
      </button>
    </div>
  );
}

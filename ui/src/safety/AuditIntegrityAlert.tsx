import type { AuditIntegrityState } from "../contracts/index";
import { describeAuditIntegrity } from "./model";
import { useCanSubmitIntent } from "../connection/read-only";

/**
 * The §15/§17 fail-closed / audit-integrity alert (safety #5) — a DISTINCT surface
 * from the transport DegradedBanner + the survival RecoveryBanner + the fencing
 * HardConflictCard. It surfaces the named treatments (partially-succeeded /
 * rollback-failed from the frozen ActionRequest enum; unknown-outcome /
 * audit-write-failed / corrupt-payload net-new) — every one with a non-color
 * channel (glyph + label + severity).
 *
 * NON-DISMISSIBLE (#5 fail-closed): the signal MUST be seen, so there is no local
 * dismiss/close control. Any acknowledge is a daemon-1.5 INTENT rendered
 * disabled/parked (the single mutator / INV-SEC-1 lives daemon-side); the
 * `canSubmitIntent` gate is pre-threaded for when it wires, but parked dominates.
 * DISPLAY only; renders nothing when there is no integrity signal (non-intrusive).
 */
export function AuditIntegrityAlert({
  integrity,
}: {
  integrity: AuditIntegrityState | null;
}) {
  const canSubmitIntent = useCanSubmitIntent();
  if (!integrity) return null;
  const desc = describeAuditIntegrity(integrity);

  return (
    <div
      className="audit-integrity-alert"
      role="alert"
      data-testid="audit-integrity-alert"
      data-treatment={desc.treatment}
      data-severity={desc.severity}
    >
      <span className="audit-integrity-alert__glyph" aria-hidden="true">
        {desc.glyph}
      </span>
      <span className="audit-integrity-alert__label">{desc.label}</span>
      <p className="audit-integrity-alert__message">{desc.message}</p>
      <button
        type="button"
        className="audit-integrity-alert__acknowledge"
        data-testid="audit-integrity-acknowledge"
        // Parked dominates today: `acknowledgeParked` is always true while the
        // acknowledge intent is deferred (daemon-1.5), so the control is disabled
        // regardless of connection. There is deliberately NO local-dismiss control
        // (#5 fail-closed — the signal must be seen); `canSubmitIntent` is pre-
        // threaded so the gate is in place when the acknowledge intent wires.
        disabled={desc.acknowledgeParked || !canSubmitIntent}
        title="Acknowledge — recorded via the daemon (daemon-1.5); the alert cannot be locally dismissed (#5 fail-closed)."
      >
        Acknowledge
      </button>
    </div>
  );
}

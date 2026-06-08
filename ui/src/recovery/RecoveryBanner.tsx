import type { RecoveryState, RecoveryStatus } from "../contracts/index";
import { describeRecovery } from "./model";

interface RecoveryBannerProps {
  recovery: RecoveryStatus;
}

// Narrowed to RecoveryState so a state rename/typo is a compile error, not a
// silent "" fallback. `recovered` has no glyph (it renders nothing).
const KIND_GLYPH: Partial<Record<RecoveryState, string>> = {
  recovering: "⟳",
  recovery_failed: "⚠",
};

/**
 * The post-restart survival banner (O-2, §11.4) — a DISTINCT surface from the
 * transport `DegradedBanner` (6.1c): session-survival vs connection state, never
 * conflated. `recovered` is non-intrusive (renders nothing). `recovery_failed`
 * surfaces the affected sessions + a "Restart session" affordance rendered
 * DISABLED/parked — the intent is deferred to daemon-1.5 (unconditionally
 * disabled = not offering a mutation, so forbidden #6 isn't triggered; the
 * canSubmitIntent gate threads when the action wires). Display only.
 */
export function RecoveryBanner({ recovery }: RecoveryBannerProps) {
  const desc = describeRecovery(recovery.state);
  if (!desc.visible) return null;

  const affected = recovery.affectedSessions ?? [];

  return (
    <div
      className="recovery-banner"
      role={desc.kind === "recovery_failed" ? "alert" : "status"}
      data-recovery-kind={desc.kind}
      data-testid="recovery-banner"
    >
      <span className="recovery-banner__glyph" aria-hidden="true">
        {KIND_GLYPH[desc.kind] ?? ""}
      </span>
      <span className="recovery-banner__message">{desc.message}</span>
      {desc.restartApplies ? (
        <>
          {affected.length > 0 ? (
            <ul className="recovery-banner__affected" data-testid="recovery-affected">
              {affected.map((id) => (
                <li key={id} data-session-id={id}>
                  {id}
                </li>
              ))}
            </ul>
          ) : null}
          <button
            type="button"
            className="recovery-banner__restart"
            data-testid="restart-session"
            disabled
            title="Restart session — available when the recovery action lands (daemon-1.5)."
          >
            Restart session
          </button>
        </>
      ) : null}
    </div>
  );
}

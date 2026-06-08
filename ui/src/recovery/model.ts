// The survival/recovery view-model (O-2, §11.4): pure mappings from the recovery
// state → a banner descriptor and from a session's resume_mode → an indicator
// descriptor. Display only — the daemon survival logic (O-2) is fixture-driven
// here, and the restart-session INTENT is parked (daemon-1.5). The §17 safety-
// state surfaces (conflict / audit-integrity) are a SEPARATE slice (6.4d-2).
import type { RecoveryState, ResumeMode } from "../contracts/index";

export interface RecoveryBannerDescriptor {
  kind: RecoveryState;
  /** Whether a banner shows at all — `recovered` is non-intrusive (false). */
  visible: boolean;
  message: string;
  /** `recovery_failed` → a (parked) "Restart session" affordance applies. */
  restartApplies: boolean;
}

export function describeRecovery(state: RecoveryState): RecoveryBannerDescriptor {
  switch (state) {
    case "recovering":
      return {
        kind: state,
        visible: true,
        message: "Recovering sessions after restart…",
        restartApplies: false,
      };
    case "recovered":
      return {
        kind: state,
        visible: false, // non-intrusive — no blocking banner
        message: "Sessions recovered.",
        restartApplies: false,
      };
    case "recovery_failed":
      return {
        kind: state,
        visible: true,
        message: "Some sessions could not be recovered after the restart.",
        restartApplies: true,
      };
  }
}

export interface ResumeModeDescriptor {
  mode: ResumeMode;
  /** Non-color channel (never color alone — §11.6). */
  glyph: string;
  label: string;
}

// resumed = brought back live; replayed = relaunched from the event log.
const RESUME_MODE: Record<ResumeMode, { glyph: string; label: string }> = {
  resumed: { glyph: "▶", label: "Resumed (live)" },
  replayed: { glyph: "⟳", label: "Replayed (relaunched)" },
};

export function describeResumeMode(mode: ResumeMode): ResumeModeDescriptor {
  return { mode, ...RESUME_MODE[mode] };
}

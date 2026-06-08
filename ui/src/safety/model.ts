// The §17 safety-state view-model (§11.4, §15/§17): pure mappings from a safety
// state → a display descriptor. DISPLAY only — the daemon §17 enforcement is
// daemon-side (INV-SEC-1 / single mutator); every resolution/acknowledge is a
// PARKED daemon-1.5 intent, rendered disabled-but-present. This is a THIRD
// distinct surface beyond transport-degraded (6.1c) + session-survival (6.4d):
// safety-state (conflict / audit-integrity). The audit-integrity half (L2)
// REUSES the frozen ActionRequest enum for partially_succeeded/rollback_failed.
import type { FencingConflict } from "../contracts/index";

/** The load-bearing safety-#6 promise — surfaced verbatim on the card. */
const NEVER_AUTO_RESOLVED =
  "This conflict requires manual resolution — it is never auto-resolved.";

export interface ConflictCardDescriptor {
  reason: FencingConflict["reason"];
  /** The affected action — surfaced verbatim, never invented (forbidden #2). */
  actionRequestId: string;
  /** The affected session, when session-scoped. */
  sessionId?: string;
  summary: string;
  /** The load-bearing safety-#6 message (manual resolution, never auto-resolved). */
  message: string;
  /** Resolution is a daemon-1.5 INTENT — parked, rendered disabled-but-present. */
  resolutionParked: true;
  /** Non-color channels (never color alone — §11.6). */
  glyph: string;
  label: string;
  severity: "critical";
}

/**
 * A fencing/hard-conflict → its card descriptor. The message states manual,
 * never-auto-resolved resolution (#6); NO auto-resolve field is produced — the
 * card offers no auto path at all. A fencing conflict is the highest safety
 * severity ("critical").
 */
export function describeConflict(conflict: FencingConflict): ConflictCardDescriptor {
  return {
    reason: conflict.reason,
    actionRequestId: conflict.action_request_id,
    sessionId: conflict.session_id,
    summary: conflict.summary,
    message: NEVER_AUTO_RESOLVED,
    resolutionParked: true,
    glyph: "⛔",
    label: "Fencing conflict",
    severity: "critical",
  };
}

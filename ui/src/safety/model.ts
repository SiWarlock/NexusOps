// The §17 safety-state view-model (§11.4, §15/§17): pure mappings from a safety
// state → a display descriptor. DISPLAY only — the daemon §17 enforcement is
// daemon-side (INV-SEC-1 / single mutator); every resolution/acknowledge is a
// PARKED daemon-1.5 intent, rendered disabled-but-present. This is a THIRD
// distinct surface beyond transport-degraded (6.1c) + session-survival (6.4d):
// safety-state (conflict / audit-integrity). The audit-integrity half (L2)
// REUSES the frozen ActionRequestStatus enum for partially_succeeded/rollback_failed.
import type {
  AuditIntegrityKind,
  AuditIntegrityState,
  AuditOutcomeStatus,
  FencingConflict,
} from "../contracts/index";

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

// ─── Audit-integrity (#5 fail-closed) ────────────────────────────────────────

export type AuditIntegritySeverity = "warning" | "critical";

/** The stable per-treatment key (the frozen status OR the provisional kind). */
export type AuditIntegrityTreatment = AuditOutcomeStatus | AuditIntegrityKind;

export interface AuditIntegrityDescriptor {
  treatment: AuditIntegrityTreatment;
  /** Non-color channels (never color alone — §11.6). */
  glyph: string;
  label: string;
  severity: AuditIntegritySeverity;
  message: string;
  /** Acknowledge is a daemon-1.5 INTENT — parked, rendered disabled-but-present. */
  acknowledgeParked: true;
}

// A treatment carries severity + label + message; the glyph is DERIVED from the
// severity (below) so the non-color channel can never drift from the severity it
// signals (§11.6 — a `critical` always reads `⛔`, a `warning` always `⚠`).
type Treatment = Omit<AuditIntegrityDescriptor, "treatment" | "acknowledgeParked" | "glyph">;

/** Glyph tracks severity by construction — never set per-treatment (§11.6). */
const SEVERITY_GLYPH: Record<AuditIntegritySeverity, string> = {
  warning: "⚠",
  critical: "⛔",
};

// The two FROZEN ActionRequestStatus outcomes (§17 event-write / rollback rows).
const ACTION_OUTCOME: Record<AuditOutcomeStatus, Treatment> = {
  partially_succeeded: {
    label: "Partially succeeded",
    severity: "warning",
    message:
      "An action partially succeeded — a side effect applied but its terminal audit event could not be fully recorded. Outcome under audit review.",
  },
  rollback_failed: {
    label: "Rollback failed",
    severity: "critical",
    message:
      "A rollback failed — a partially-applied action could not be reverted. Manual audit review required.",
  },
};

// The net-new PROVISIONAL integrity signals (§17 daemon-crash / event-write /
// corrupt-payload rows). All are critical — un-reconcilable or audit-breaking.
const INTEGRITY: Record<AuditIntegrityKind, Treatment> = {
  unknown_outcome: {
    label: "Unknown outcome",
    severity: "critical",
    message:
      "The daemon stopped mid-action and the outcome is un-reconcilable — recorded as unknown.",
  },
  audit_write_failed: {
    label: "Audit write failed",
    severity: "critical",
    message:
      "An audit-required action's authoritative event could not be written — the action was failed closed (safety #5).",
  },
  corrupt_payload: {
    label: "Corrupt audit payload",
    severity: "critical",
    message:
      "A corrupt event payload was quarantined and an audit-integrity event recorded.",
  },
};

/**
 * A fail-closed / audit-integrity state → its alert descriptor. The `action_status`
 * branch reuses the FROZEN ActionRequestStatus outcomes; the `integrity` branch carries
 * the provisional net-new signals. Every treatment renders a non-color channel
 * (glyph + label + severity), glyph derived from severity — #5 means the signal
 * must be seen, never color-only.
 */
export function describeAuditIntegrity(
  state: AuditIntegrityState,
): AuditIntegrityDescriptor {
  const treatment = state.source === "action_status" ? state.status : state.kind;
  const t =
    state.source === "action_status"
      ? ACTION_OUTCOME[state.status]
      : INTEGRITY[state.kind];
  return { treatment, acknowledgeParked: true, glyph: SEVERITY_GLYPH[t.severity], ...t };
}

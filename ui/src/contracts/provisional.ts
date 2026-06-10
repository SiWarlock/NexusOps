// PROVISIONAL — not frozen; reconciles when the daemon freezes object schemas
// (Phase 1/2 contract bump). The 0.5 freeze is ENUM-ONLY: projection-row shapes,
// GatewayPort params/results, and the ActionRequest/ActionPlan OBJECTS are
// Appendix-A prose, not yet a generated artifact. Until they freeze we type them
// here as minimal UI-local shapes — with EVERY enum-typed field delegated to the
// GENERATED zod enums (never a re-declared status string union). See ARCHITECTURE
// §5.0 / §6.1 / §7.2 and the brief's Step-2.5 Q2.
import { z } from "zod";
import bundle from "./generated";

// Delegate to the generated enums (no hand-declared status unions).
const Session = bundle.shape.Session;
const PullRequest = bundle.shape.PullRequest;
const Approval = bundle.shape.Approval;
const ActorType = bundle.shape.ActorType;
const ActionRequest = bundle.shape.ActionRequest;

// ─── Survival/recovery (PROVISIONAL — 6.4d) ──────────────────────────────────
// The daemon's O-2 survival schema is NOT frozen. RecoveryState/ResumeMode/
// RecoveryStatus are hand-declared provisional shapes (Lesson §2); they reconcile
// at the daemon survival-schema freeze (Carry-forward provisional→generated spread).

/** Post-restart recovery state (O-2, §11.4). PROVISIONAL. */
export const RecoveryState = z.enum(["recovering", "recovered", "recovery_failed"]);
export type RecoveryState = z.infer<typeof RecoveryState>;

/** How a session was brought back after a restart (O-2): live vs relaunched. PROVISIONAL. */
export const ResumeMode = z.enum(["resumed", "replayed"]);
export type ResumeMode = z.infer<typeof ResumeMode>;

/** The recovery status input (fixture-driven; daemon survival logic not built). */
export const RecoveryStatus = z.object({
  state: RecoveryState,
  // sessions affected by a failed recovery (subject ids); empty/absent otherwise.
  affectedSessions: z.array(z.string()).optional(),
});
export type RecoveryStatus = z.infer<typeof RecoveryStatus>;

// ─── §17 safety-state (PROVISIONAL — 6.4d-2) ─────────────────────────────────
// The daemon's §17 failure-mode / conflict schema is NOT in the frozen contract.
// ConflictReason + FencingConflict are hand-declared provisional shapes (Lesson
// §2); they reconcile at the daemon §17/survival-schema freeze (Carry-forward
// provisional→generated spread). The audit-integrity treatments (L2) REUSE the
// frozen ActionRequest enum for partially_succeeded/rollback_failed — never
// re-declared here.

/**
 * Why a mutation was rejected at the fencing gate (§17 fencing row, safety #6).
 * Scoped to `fencing_conflict` ONLY — the never-auto-resolved hard conflict.
 * `stale_precondition` is a SEPARATE, RE-APPROVABLE flow (regenerate preview +
 * require fresh approval, §17/§6.2) — NOT a hard conflict; it belongs with the
 * approval/preview surface, not this card, so it is deliberately excluded here.
 * PROVISIONAL.
 */
export const ConflictReason = z.enum(["fencing_conflict"]);
export type ConflictReason = z.infer<typeof ConflictReason>;

/**
 * A stale-token / lease-expiry rejected mutation → `ActionFailed(fencing_conflict)`
 * + conflict event (§17). The UI DISPLAYS it as the never-auto-resolved hard-
 * conflict card (safety #6); resolution is a daemon-side typed Action (parked,
 * daemon-1.5 — single mutator / INV-SEC-1 is daemon-side). PROVISIONAL.
 */
export const FencingConflict = z.object({
  // The affected action (frozen IdKind `action_request_id`; the value is the `act_…` id string).
  action_request_id: z.string(),
  // The affected session when session-scoped (frozen IdKind `session_id`).
  session_id: z.string().optional(),
  reason: ConflictReason,
  // Human-readable summary of what was rejected (daemon-supplied; never invented — forbidden #2).
  summary: z.string(),
});
export type FencingConflict = z.infer<typeof FencingConflict>;

// ─── §17 audit-integrity (PROVISIONAL — 6.4d-2 L2) ───────────────────────────
// Fail-closed / audit-integrity treatments (§11.4, §15/§17, safety #5). The two
// action-outcome treatments REUSE the frozen ActionRequest enum (drift-pinned via
// `.extract` — it THROWS at load if the frozen enum renames them; never a re-
// declaration, Lesson §2). Only the net-new integrity signals are provisional.

/** The two frozen ActionRequest outcomes that raise an audit-integrity alert. */
export const AuditOutcomeStatus = ActionRequest.extract([
  "partially_succeeded",
  "rollback_failed",
]);
export type AuditOutcomeStatus = z.infer<typeof AuditOutcomeStatus>;

/** Net-new audit-integrity signals NOT in the frozen contract (§17). PROVISIONAL. */
export const AuditIntegrityKind = z.enum([
  "unknown_outcome",
  "audit_write_failed",
  "corrupt_payload",
]);
export type AuditIntegrityKind = z.infer<typeof AuditIntegrityKind>;

/**
 * A fail-closed / audit-integrity signal (§15/§17, safety #5) — a discriminated
 * union: `action_status` reuses the frozen ActionRequest outcome enum, `integrity`
 * carries the provisional net-new signals (daemon-crash unknown-outcome, event-
 * write-fail, corrupt-payload quarantine). PROVISIONAL.
 */
export const AuditIntegrityState = z.discriminatedUnion("source", [
  z.object({ source: z.literal("action_status"), status: AuditOutcomeStatus }),
  z.object({ source: z.literal("integrity"), kind: AuditIntegrityKind }),
]);
export type AuditIntegrityState = z.infer<typeof AuditIntegrityState>;

/**
 * The aggregate §17 safety state the Shell renders (fixture-driven; the daemon
 * §17 failure-mode logic isn't built). PROVISIONAL — mirrors the `RecoveryStatus`
 * input-prop shape (a Zod schema, validated at the boundary when real state lands).
 */
export const SafetyState = z.object({
  // A pending fencing/hard-conflict, or null when clean (non-intrusive default).
  conflict: FencingConflict.nullable(),
  // A pending fail-closed/audit-integrity signal, or null when clean. REQUIRED
  // (not optional) so a #5 caller can't silently omit a must-be-seen alert.
  integrity: AuditIntegrityState.nullable(),
});
export type SafetyState = z.infer<typeof SafetyState>;

/** A single row of the Session projection (provisional shape). */
export const SessionRow = z.object({
  session_id: z.string(),
  status: Session,
  title: z.string().optional(),
  project_id: z.string().optional(),
  // O-2 survival display (PROVISIONAL): how the session came back after a restart.
  resume_mode: ResumeMode.optional(),
});
export type SessionRow = z.infer<typeof SessionRow>;

/** A page of the Session projection returned by get_projection (provisional). */
export const SessionProjectionPage = z.object({
  projection: z.literal("Session"),
  rows: z.array(SessionRow),
  cursor: z.string().nullable().optional(),
});
export type SessionProjectionPage = z.infer<typeof SessionProjectionPage>;

/** A single row of the ProjectActivity projection — a project (provisional). */
export const ProjectActivityRow = z.object({
  project_id: z.string(),
  name: z.string(),
});
export type ProjectActivityRow = z.infer<typeof ProjectActivityRow>;

export const ProjectActivityPage = z.object({
  projection: z.literal("ProjectActivity"),
  rows: z.array(ProjectActivityRow),
  cursor: z.string().nullable().optional(),
});
export type ProjectActivityPage = z.infer<typeof ProjectActivityPage>;

/** A row of the PullRequest projection (provisional; status delegates to the frozen enum). */
export const PullRequestRow = z.object({
  pr_number: z.string(),
  project_id: z.string(),
  status: PullRequest,
  title: z.string().optional(),
});
export type PullRequestRow = z.infer<typeof PullRequestRow>;

export const PullRequestProjectionPage = z.object({
  projection: z.literal("PullRequest"),
  rows: z.array(PullRequestRow),
  cursor: z.string().nullable().optional(),
});
export type PullRequestProjectionPage = z.infer<typeof PullRequestProjectionPage>;

/** A row of the ApprovalQueue projection (provisional; status delegates to the frozen enum). */
export const ApprovalQueueRow = z.object({
  approval_id: z.string(),
  project_id: z.string(),
  status: Approval,
  title: z.string().optional(),
});
export type ApprovalQueueRow = z.infer<typeof ApprovalQueueRow>;

export const ApprovalQueuePage = z.object({
  projection: z.literal("ApprovalQueue"),
  rows: z.array(ApprovalQueueRow),
  cursor: z.string().nullable().optional(),
});
export type ApprovalQueuePage = z.infer<typeof ApprovalQueuePage>;

/** A row of the AuditTrail projection — one event (provisional; actor delegates to the frozen enum). */
export const AuditEventRow = z.object({
  event_id: z.string(),
  seq: z.number(),
  project_id: z.string().optional(),
  actor_type: ActorType,
  event_type: z.string(),
  summary: z.string().optional(),
});
export type AuditEventRow = z.infer<typeof AuditEventRow>;

export const AuditTrailPage = z.object({
  projection: z.literal("AuditTrail"),
  rows: z.array(AuditEventRow),
  cursor: z.string().nullable().optional(),
});
export type AuditTrailPage = z.infer<typeof AuditTrailPage>;

// ─── Usage (PROVISIONAL — 6.4b) ──────────────────────────────────────────────
// The daemon's usage/telemetry schema is NOT in the frozen contract. UsageRow +
// metric_quality + Harness are hand-declared provisional shapes (Lesson §2); they
// (incl. the credit-pool thresholds + enum delegation) reconcile at the daemon
// usage-schema freeze — tracked in the MVP_TASKS Carry-forward provisional→generated
// spread. No frozen MetricQuality/Harness enum exists yet, so these are local.

/** Adapter-reported accuracy of a usage metric (§9.1 metric_quality). PROVISIONAL. */
export const MetricQuality = z.enum(["exact", "estimated", "unavailable"]);
export type MetricQuality = z.infer<typeof MetricQuality>;

/** The agent harness a session runs under. PROVISIONAL (no frozen Harness enum). */
export const Harness = z.enum(["claude", "codex"]);
export type Harness = z.infer<typeof Harness>;

/** Per-subject token/cost usage (§11.4). PROVISIONAL. */
export const UsageRow = z.object({
  subject_id: z.string(),
  harness: Harness,
  tokens: z.number(),
  cost: z.number(),
  metric_quality: MetricQuality,
  // Codex reports no context metadata (§9.1 supportsContextMetadata=false) → null.
  context_pct: z.number().nullable().optional(),
});
export type UsageRow = z.infer<typeof UsageRow>;

/** The capped Agent-SDK credit pool (hard-stops — §0.1), distinct from token spend. */
export const CreditPool = z.object({
  used: z.number(),
  limit: z.number(),
});
export type CreditPool = z.infer<typeof CreditPool>;

export const UsageProjectionPage = z.object({
  // 0.12.0: the frozen ProjectionName enum fixes the canonical name as
  // "UsageLedger" (was provisional "Usage").
  projection: z.literal("UsageLedger"),
  rows: z.array(UsageRow),
  creditPool: CreditPool.nullable().optional(),
  cursor: z.string().nullable().optional(),
});
export type UsageProjectionPage = z.infer<typeof UsageProjectionPage>;

/**
 * Typed projection registry (provisional). Maps each projection name to its page
 * shape so get_projection / parseProjectionPage give precise per-name types.
 */
export type ProjectionPageByName = {
  Session: SessionProjectionPage;
  ProjectActivity: ProjectActivityPage;
  PullRequest: PullRequestProjectionPage;
  ApprovalQueue: ApprovalQueuePage;
  AuditTrail: AuditTrailPage;
  UsageLedger: UsageProjectionPage;
};
export type ProjectionName = keyof ProjectionPageByName;

/** Any projection page (the union over the registry). */
export type ProjectionPage = ProjectionPageByName[ProjectionName];

/** A streamed projection delta from subscribe (provisional; kind delegates to
 *  the frozen 0.12.0 DeltaKind enum — never re-declared, Lesson §1/§2). */
export const ProjectionDelta = z.object({
  projection: z.string(),
  kind: bundle.shape.DeltaKind,
  row: SessionRow.optional(),
  id: z.string().optional(),
});
export type ProjectionDelta = z.infer<typeof ProjectionDelta>;

/** get_capabilities result (provisional; §6.4 handshake surface). */
export const Capabilities = z.object({
  protocol_version: z.number(),
  contract_version: z.string(),
});
export type Capabilities = z.infer<typeof Capabilities>;

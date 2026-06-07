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

/** A single row of the Session projection (provisional shape). */
export const SessionRow = z.object({
  session_id: z.string(),
  status: Session,
  title: z.string().optional(),
  project_id: z.string().optional(),
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
};
export type ProjectionName = keyof ProjectionPageByName;

/** Any projection page (the union over the registry). */
export type ProjectionPage = ProjectionPageByName[ProjectionName];

/** A streamed projection delta from subscribe (provisional). */
export const ProjectionDelta = z.object({
  projection: z.string(),
  kind: z.enum(["upsert", "remove"]),
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

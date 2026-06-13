// Intent mutation shapes (§6.1/§6.2) — PROVISIONAL frozen-shadows.
//
// The §6 Gateway OBJECT models (ActionRequest/ActionAck/ActionPreview/Approval) are
// frozen in the schema, but the §5.0 generator emits flat enum $defs only — so these
// objects are hand-modeled provisional SHADOWS here (Lesson §2), with every enum-typed
// field DELEGATED to the generated bundle (never re-declared). They are drift-pinned to
// the frozen schema's field-sets by intent/submit-intent.test (the §2.5-seam snapshot,
// the same pattern as ServerFrame). They live in contracts/ (not intent/) so the
// gateway-client typing them stays a downward dependency. The seam is a pure CLIENT of
// this frozen contract — it never invents a shape (forbidden #2); the daemon's Action
// Gateway is the authority.
import { z } from "zod";
import bundle from "./generated";

// RiskLevel is a frozen INTEGER bounded 0–4 (the §6.2 risk axis), NOT an enum — the
// policy engine is catalog-authoritative; the UI never derives it (Q4). Timestamp is RFC3339.
const RiskLevel = z.number().int().min(0).max(4);
const Timestamp = z.string();

/** §6.2 ResourceRef — a targeted resource. Enum field delegated. */
export const ResourceRef = z.object({
  type: bundle.shape.ResourceType,
  id: z.string(),
  uri: z.string().nullable().optional(),
});
export type ResourceRef = z.infer<typeof ResourceRef>;

/** §6.2 ActorRefBody — a specific approving actor (the `actor` of a RequiredApprover). */
export const ActorRefBody = z.object({
  actor_type: bundle.shape.RequesterType,
  actor_id: z.string(),
});
export type ActorRefBody = z.infer<typeof ActorRefBody>;

/** §6.2 RequiredApprover — which class of approver an Approval requires. `actor` is the
 *  frozen `anyOf:[ActorRefBody, null]` (a structured actor, NOT a bare string). */
export const RequiredApprover = z.object({
  kind: bundle.shape.RequiredApproverKind,
  actor: ActorRefBody.nullable().optional(),
});
export type RequiredApprover = z.infer<typeof RequiredApprover>;

/** §6.1 submit_action ack — the daemon mints `action_request_id` + reports `status`. */
export const ActionAck = z.object({
  action_request_id: z.string(),
  status: bundle.shape.ActionRequestStatus,
});
export type ActionAck = z.infer<typeof ActionAck>;

/** §6.2 ActionPreview — the daemon's consequence preview (never UI-synthesized, #2). */
export const ActionPreview = z.object({
  action_request_id: z.string(),
  generated_at: Timestamp,
  risk_level: RiskLevel,
  risk_reasons: z.array(z.string()),
  summary: z.string(),
  changed_resources: z.array(ResourceRef),
  cannot_preview_reason: z.string().nullable().optional(),
});
export type ActionPreview = z.infer<typeof ActionPreview>;

/** §6.2 Approval — the human/policy decision. The UI submits it; the daemon's policy
 *  engine owns the risk/requirement (Q4 — no UI-derived risk). */
export const Approval = z.object({
  approval_id: z.string(),
  required_approver: RequiredApprover,
  status: bundle.shape.ApprovalStatus,
  scope: bundle.shape.ApprovalScope,
  risk_level: RiskLevel,
  action_request_id: z.string().nullable().optional(),
  decided_at: Timestamp.nullable().optional(),
  decided_by: z.string().nullable().optional(),
  expires_at: Timestamp.nullable().optional(),
  plan_id: z.string().nullable().optional(),
});
export type Approval = z.infer<typeof Approval>;

/** §6.2 ActionRequest — the intent the UI submits. The daemon owns lifecycle `status`,
 *  `risk_level`, `idempotency_key`, `fencing_token` (the UI never computes them). */
export const ActionRequest = z.object({
  action_request_id: z.string(),
  action_type: z.string(),
  requester_type: bundle.shape.RequesterType,
  requester_id: z.string(),
  resource_refs: z.array(ResourceRef),
  inputs: z.unknown(),
  risk_level: RiskLevel,
  status: bundle.shape.ActionRequestStatus,
  created_at: Timestamp,
  idempotency_key: z.string().nullable().optional(),
  fencing_token: z.number().int().nullable().optional(),
  preview: ActionPreview.nullable().optional(),
  project_id: z.string().nullable().optional(),
});
export type ActionRequest = z.infer<typeof ActionRequest>;

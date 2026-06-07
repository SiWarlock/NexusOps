// PROVISIONAL — not frozen; reconciles when the daemon freezes object schemas
// (Phase 1/2 contract bump). The 0.5 freeze is ENUM-ONLY: projection-row shapes,
// GatewayPort params/results, and the ActionRequest/ActionPlan OBJECTS are
// Appendix-A prose, not yet a generated artifact. Until they freeze we type them
// here as minimal UI-local shapes — with EVERY enum-typed field delegated to the
// GENERATED zod enums (never a re-declared status string union). See ARCHITECTURE
// §5.0 / §6.1 / §7.2 and the brief's Step-2.5 Q2.
import { z } from "zod";
import bundle from "./generated";

// Delegate to the generated Session enum (no hand-declared status union).
const Session = bundle.shape.Session;

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

/**
 * Generic projection page (provisional). Only Session exists in 6.1a; this
 * alias widens to a union as more projections land in later slices.
 */
export type ProjectionPage = SessionProjectionPage;

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

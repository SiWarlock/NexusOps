// The parse-don't-trust boundary validator.
//
// EVERY projection payload crossing the daemon boundary is Zod-validated here
// BEFORE it reaches view logic (ui/CLAUDE.md typing posture; §4.2 law 2). A
// malformed payload (unknown status / missing required field) FAILS CLOSED — it
// throws and the malformed value is never returned to a caller (§15/§17
// fail-closed; ui/CLAUDE.md forbidden #2/#3).
//
// Boundary failures throw a typed BoundaryValidationError carrying the
// projection name + the underlying ZodError, so the 6.1b shell can catch it and
// branch into read-only / degraded mode with structured context.
import type { ZodError } from "zod";
import {
  ApprovalQueuePage,
  AuditTrailPage,
  Capabilities,
  DiffResult,
  ProjectActivityPage,
  ProjectionDelta,
  PullRequestProjectionPage,
  SessionProjectionPage,
  UsageProjectionPage,
  type ProjectionPage,
} from "../contracts/index";

export class BoundaryValidationError extends Error {
  readonly projection: string;
  readonly zodError: ZodError;

  constructor(projection: string, zodError: ZodError) {
    super(
      `boundary validation failed for projection "${projection}": ${zodError.message}`,
    );
    this.name = "BoundaryValidationError";
    this.projection = projection;
    this.zodError = zodError;
  }
}

/** Provisional registry: projection name → its boundary schema. */
const PAGE_SCHEMAS = {
  Session: SessionProjectionPage,
  ProjectActivity: ProjectActivityPage,
  PullRequest: PullRequestProjectionPage,
  ApprovalQueue: ApprovalQueuePage,
  AuditTrail: AuditTrailPage,
  UsageLedger: UsageProjectionPage,
} as const;

/**
 * Validate a projection page at the boundary. Throws BoundaryValidationError on
 * a malformed payload; the malformed value is never returned.
 */
export function parseProjectionPage(
  name: string,
  payload: unknown,
): ProjectionPage {
  const schema = PAGE_SCHEMAS[name as keyof typeof PAGE_SCHEMAS];
  if (!schema) {
    // An unregistered projection name is a UI programmer error (the UI asked
    // for a projection it has no schema for), NOT a malformed-daemon-payload
    // condition — so it fails LOUD with a plain Error rather than a
    // BoundaryValidationError that 6.1b would funnel into degraded mode.
    throw new Error(
      `parseProjectionPage: no boundary schema registered for projection "${name}"`,
    );
  }
  const result = schema.safeParse(payload);
  if (!result.success) {
    throw new BoundaryValidationError(name, result.error);
  }
  return result.data;
}

/** Validate a streamed projection delta at the boundary (same fail-closed posture). */
export function parseDelta(payload: unknown): ProjectionDelta {
  const result = ProjectionDelta.safeParse(payload);
  if (!result.success) {
    throw new BoundaryValidationError("delta", result.error);
  }
  return result.data;
}

/** Validate a `get_diff` result at the boundary (same fail-closed posture). The real
 *  UdsGatewayPort gets this from the daemon over the wire — parse-don't-trust before
 *  it reaches the Code/Diff view. */
export function parseDiff(payload: unknown): DiffResult {
  const result = DiffResult.safeParse(payload);
  if (!result.success) {
    throw new BoundaryValidationError("diff", result.error);
  }
  return result.data;
}

/** Validate a `get_capabilities` result at the boundary (same fail-closed posture);
 *  feeds the §6.4/§16 version-compat check. */
export function parseCapabilities(payload: unknown): Capabilities {
  const result = Capabilities.safeParse(payload);
  if (!result.success) {
    throw new BoundaryValidationError("capabilities", result.error);
  }
  return result.data;
}
